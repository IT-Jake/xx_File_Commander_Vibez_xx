use eframe::egui;
use std::path::PathBuf;
use crate::fs::{list_files, FileItem};

pub struct Pane {
    current_path: PathBuf,
    items: Vec<FileItem>,
    selected_item: Option<PathBuf>,
    error_msg: Option<String>,
    renaming_item: Option<PathBuf>,
    rename_buffer: String,
    open_with_item: Option<PathBuf>,
    open_with_buffer: String,
    available_apps: Vec<PathBuf>,
    filtered_apps: Vec<PathBuf>,
}

impl Pane {
    pub fn new(_title: &str, path: PathBuf) -> Self {
        let mut pane = Self {
            current_path: path,
            items: Vec::new(),
            selected_item: None,
            error_msg: None,
            renaming_item: None,
            rename_buffer: String::new(),
            open_with_item: None,
            open_with_buffer: String::new(),
            available_apps: Vec::new(),
            filtered_apps: Vec::new(),
        };
        pane.refresh();
        pane
    }

    pub fn refresh(&mut self) {
        match list_files(&self.current_path) {
            Ok(items) => {
                self.items = items;
                self.error_msg = None;
            }
            Err(e) => {
                self.error_msg = Some(e.to_string());
                self.items.clear();
            }
        }
    }

    fn scan_apps(&mut self) {
        self.available_apps.clear();
        let paths = ["/Applications", "/System/Applications", "/Users/jake/Applications"];
        
        for base_path in paths {
            if let Ok(entries) = std::fs::read_dir(base_path) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.extension().and_then(|s| s.to_str()) == Some("app") {
                        self.available_apps.push(path);
                    }
                }
            }
        }
        self.available_apps.sort();
        self.filtered_apps = self.available_apps.clone();
    }

    pub fn ui(&mut self, ui: &mut egui::Ui, clipboard: &mut Option<crate::app::ClipboardState>, pane_id: &str) {
        ui.vertical(|ui| {
            // Header
            ui.label(self.current_path.to_string_lossy());

            // Navigation Buttons
            ui.horizontal(|ui| {
                if ui.button("Up").clicked() {
                    if let Some(parent) = self.current_path.parent() {
                        if parent.exists() {
                            self.current_path = parent.to_path_buf();
                            self.refresh();
                        }
                    }
                }
                if ui.button("Refresh").clicked() {
                    self.refresh();
                }
                
                if ui.button("Delete").clicked() {
                    if let Some(path) = &self.selected_item {
                        if let Err(e) = trash::delete(path) {
                            self.error_msg = Some(format!("Delete failed: {}", e));
                        } else {
                            self.refresh();
                            self.selected_item = None;
                        }
                    }
                }

                if ui.button("Copy Path").clicked() {
                    if let Some(path) = &self.selected_item {
                         if let Ok(mut clipboard) = arboard::Clipboard::new() {
                             let _ = clipboard.set_text(path.to_string_lossy().to_string());
                         }
                    }
                }
            });

            if let Some(err) = &self.error_msg {
                ui.colored_label(egui::Color32::RED, err);
            }

            // File List
            egui::ScrollArea::vertical()
                .id_source(pane_id)
                .auto_shrink([false, false])
                .show(ui, |ui| {
                // We need to collect actions to perform after the loop to avoid borrowing issues
                let mut navigate_to = None;

                for item in &self.items {
                    let icon = if item.is_dir { "📁" } else { "📄" };
                    // Format size
                    let size_str = if item.is_dir {
                        "".to_string()
                    } else {
                        crate::fs::format_size(item.size)
                    };
                    
                    let label = format!("{} {}  |  {}  |  {}", icon, item.name, size_str, item.modified);
                    
                    let is_selected = Some(item.path.clone()) == self.selected_item;
                    
                    let response = ui.selectable_label(is_selected, label);
                    
                    // Drag Source
                    let response = response.interact(egui::Sense::drag());
                    if response.drag_started() {
                        // Set payload manually using temp data
                        ui.ctx().data_mut(|d| d.insert_temp(egui::Id::new("drag_payload"), item.path.clone()));
                    }
                    
                    if response.double_clicked() {
                        if item.is_dir {
                            navigate_to = Some(item.path.clone());
                        } else {
                            let _ = open::that(&item.path);
                        }
                    } else if response.clicked() {
                         self.selected_item = Some(item.path.clone());
                    }
                    
                    // Context Menu
                    response.context_menu(|ui| {
                        if ui.button("Cut").clicked() {
                            *clipboard = Some(crate::app::ClipboardState {
                                op: crate::app::ClipboardOp::Cut,
                                path: item.path.clone(),
                            });
                            ui.close_menu();
                        }
                        if ui.button("Copy").clicked() {
                            *clipboard = Some(crate::app::ClipboardState {
                                op: crate::app::ClipboardOp::Copy,
                                path: item.path.clone(),
                            });
                            ui.close_menu();
                        }
                        if ui.button("Delete").clicked() {
                             if let Err(e) = trash::delete(&item.path) {
                                // We can't easily set error_msg from here due to borrowing, 
                                // but we could log it or handle it next frame. 
                                // For now, simple print.
                                eprintln!("Delete failed: {}", e);
                            }
                            ui.close_menu();
                            // Refresh needed, but we can't call self.refresh() here easily.
                            // We'll rely on the main loop or manual refresh for now, 
                            // or trigger a flag.
                        }
                        if ui.button("Rename").clicked() {
                             self.renaming_item = Some(item.path.clone());
                             self.rename_buffer = item.name.clone();
                             ui.close_menu();
                        }
                        if ui.button("Open With...").clicked() {
                             self.open_with_item = Some(item.path.clone());
                             self.open_with_buffer = String::new();
                             // We can't call scan_apps here due to borrowing self.items
                             // So we'll trigger it by checking if open_with_item is set later
                             ui.close_menu();
                        }
                        
                        ui.separator();
                        
                        if let Some(clip) = clipboard {
                             if ui.button("Paste Here").clicked() {
                                 let dest = self.current_path.join(clip.path.file_name().unwrap());
                                 match clip.op {
                                     crate::app::ClipboardOp::Copy => {
                                         if clip.path.is_dir() {
                                             if let Err(e) = crate::fs::recursive_copy(&clip.path, &dest) {
                                                 eprintln!("Recursive copy failed: {}", e);
                                             }
                                         } else {
                                             if let Err(e) = std::fs::copy(&clip.path, &dest) {
                                                 eprintln!("Copy failed: {}", e);
                                             }
                                         }
                                     },
                                     crate::app::ClipboardOp::Cut => {
                                         if let Err(e) = std::fs::rename(&clip.path, &dest) {
                                             eprintln!("Move failed: {}", e);
                                         } else {
                                             *clipboard = None; 
                                         }
                                     }
                                 }
                                 ui.close_menu();
                                 // We need to signal refresh. For now, we rely on the next frame or user action, 
                                 // but let's try to force it if possible or just accept it might delay one frame.
                                 // Actually, since we are in the loop, we can't easily mutate self.items immediately 
                                 // without re-reading. But the next update loop will catch it if we trigger a repaint.
                                 ui.ctx().request_repaint();
                             }
                        }
                    });
                }
                
                // Background Context Menu (Paste, New Folder)
                ui.interact(ui.min_rect(), ui.id(), egui::Sense::click()).context_menu(|ui| {
                     if let Some(clip) = clipboard {
                         if ui.button("Paste").clicked() {
                             let dest = self.current_path.join(clip.path.file_name().unwrap());
                             match clip.op {
                                 crate::app::ClipboardOp::Copy => {
                                     // Simple copy for now (files only, directories need recursive)
                                     if clip.path.is_dir() {
                                         if let Err(e) = crate::fs::recursive_copy(&clip.path, &dest) {
                                             eprintln!("Recursive copy failed: {}", e);
                                         }
                                     } else {
                                         if let Err(e) = std::fs::copy(&clip.path, &dest) {
                                             eprintln!("Copy failed: {}", e);
                                         }
                                     }
                                 },
                                 crate::app::ClipboardOp::Cut => {
                                     if let Err(e) = std::fs::rename(&clip.path, &dest) {
                                         eprintln!("Move failed: {}", e);
                                     } else {
                                         *clipboard = None; // Clear clipboard after move
                                     }
                                 }
                             }
                             ui.close_menu();
                             // Trigger refresh
                         }
                     }
                     if ui.button("New Folder").clicked() {
                         let new_folder = self.current_path.join("New Folder");
                         if let Err(e) = std::fs::create_dir(&new_folder) {
                             eprintln!("Create dir failed: {}", e);
                         }
                         ui.close_menu();
                     }
                });

                if let Some(path) = navigate_to {
                    self.current_path = path;
                    self.refresh();
                }
            });
            
            // Handle External Drops
            if ui.rect_contains_pointer(ui.min_rect()) {
                // Internal Drop
                // Check if we have a payload
                if let Some(source_path) = ui.ctx().data(|d| d.get_temp::<std::path::PathBuf>(egui::Id::new("drag_payload"))) {
                     // If mouse is released, perform the drop
                     if ui.input(|i| i.pointer.any_released()) {
                         let dest = self.current_path.join(source_path.file_name().unwrap());
                         // Avoid copying into itself
                         if dest != *source_path {
                             if source_path.is_dir() {
                                 if let Err(e) = crate::fs::recursive_copy(&source_path, &dest) {
                                     eprintln!("Internal drop copy failed: {}", e);
                                 }
                             } else {
                                 if let Err(e) = std::fs::copy(&*source_path, &dest) {
                                     eprintln!("Internal drop copy failed: {}", e);
                                 }
                             }
                             self.refresh();
                             // Clear payload
                             ui.ctx().data_mut(|d| d.remove_temp::<std::path::PathBuf>(egui::Id::new("drag_payload")));
                         }
                     }
                }

                let dropped_files = ui.ctx().input(|i| i.raw.dropped_files.clone());
                if !dropped_files.is_empty() {
                    for file in dropped_files {
                        if let Some(path) = file.path {
                            let dest = self.current_path.join(path.file_name().unwrap());
                            if path.is_dir() {
                                if let Err(e) = crate::fs::recursive_copy(&path, &dest) {
                                    eprintln!("Drop copy failed: {}", e);
                                }
                            } else {
                                if let Err(e) = std::fs::copy(&path, &dest) {
                                    eprintln!("Drop copy failed: {}", e);
                                }
                            }
                        }
                    }
                    self.refresh();
                }
            }
        });


        // Rename Dialog
        if let Some(path) = self.renaming_item.clone() {
            let mut open = true;
            egui::Window::new("Rename")
                .id(egui::Id::new(pane_id).with("rename_dialog"))
                .open(&mut open)
                .collapsible(false)
                .resizable(false)
                .show(ui.ctx(), |ui| {
                    ui.label(format!("Rename '{}' to:", path.file_name().unwrap_or_default().to_string_lossy()));
                    let response = ui.text_edit_singleline(&mut self.rename_buffer);
                    if response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                         // Trigger rename
                    }
                    
                    ui.horizontal(|ui| {
                        if ui.button("Rename").clicked() || (response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter))) {
                            let new_path = path.parent().unwrap().join(&self.rename_buffer);
                            if let Err(e) = std::fs::rename(&path, &new_path) {
                                self.error_msg = Some(format!("Rename failed: {}", e));
                            } else {
                                self.renaming_item = None;
                                self.refresh();
                            }
                        }
                        if ui.button("Cancel").clicked() {
                            self.renaming_item = None;
                        }
                    });
                });
            
            if !open {
                self.renaming_item = None;
            }
        }
        // Open With Dialog
        if let Some(path) = self.open_with_item.clone() {
            // Lazy load apps if empty
            if self.available_apps.is_empty() {
                self.scan_apps();
            }
            
            let mut open = true;
            egui::Window::new("Open With")
                .id(egui::Id::new(pane_id).with("open_with_dialog"))
                .open(&mut open)
                .collapsible(false)
                .resizable(false)
                .show(ui.ctx(), |ui| {
                    ui.label(format!("Open '{}' with:", path.file_name().unwrap_or_default().to_string_lossy()));
                    
                    let response = ui.text_edit_singleline(&mut self.open_with_buffer);
                    if response.changed() {
                        let filter = self.open_with_buffer.to_lowercase();
                        self.filtered_apps = self.available_apps.iter()
                            .filter(|p| p.file_name().unwrap_or_default().to_string_lossy().to_lowercase().contains(&filter))
                            .cloned()
                            .collect();
                    }

                    ui.separator();

                    egui::ScrollArea::vertical().max_height(200.0).show(ui, |ui| {
                        for app_path in &self.filtered_apps {
                            let app_name = app_path.file_stem().unwrap_or_default().to_string_lossy();
                            if ui.button(app_name).clicked() {
                                if let Err(e) = open::with(&path, app_path.to_string_lossy().to_string()) {
                                    self.error_msg = Some(format!("Open failed: {}", e));
                                }
                                self.open_with_item = None;
                            }
                        }
                    });
                    
                    ui.separator();
                    
                    ui.horizontal(|ui| {
                        if ui.button("Cancel").clicked() {
                            self.open_with_item = None;
                        }
                    });
                });
            
            if !open {
                self.open_with_item = None;
            }
        }
    }
}
