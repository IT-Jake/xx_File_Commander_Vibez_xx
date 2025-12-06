use eframe::egui;
use std::path::PathBuf;
use crate::pane::Pane;

#[derive(Clone, PartialEq)]
pub enum ClipboardOp {
    Copy,
    Cut,
}

pub struct ClipboardState {
    pub op: ClipboardOp,
    pub path: PathBuf,
}

pub struct FileManagerApp {
    left_pane: Pane,
    right_pane: Pane,
    clipboard: Option<ClipboardState>,
}

impl FileManagerApp {
    pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        // Initialize panes with home directory
        let default_path = if let Some(user_dirs) = directories::UserDirs::new() {
            user_dirs.home_dir().to_path_buf()
        } else {
            std::env::current_dir().unwrap_or_default()
        };
        
        Self {
            left_pane: Pane::new("Left Pane", default_path.clone()),
            right_pane: Pane::new("Right Pane", default_path),
            clipboard: None,
        }
    }
}

impl eframe::App for FileManagerApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.columns(2, |columns| {
                self.left_pane.ui(&mut columns[0], &mut self.clipboard, "left");
                self.right_pane.ui(&mut columns[1], &mut self.clipboard, "right");
            });
        });
    }
}
