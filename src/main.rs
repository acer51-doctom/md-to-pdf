use eframe::egui;
use std::process::Command;
use std::fs;
use std::path::{Path, PathBuf};
use rfd::FileDialog; // Import the FileDialog crate

/// Enum to represent the different CSS themes
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
enum Theme {
    GitHubLight,
    GitHubDark,
    GitHubAuto,
}

impl Theme {
    /// Returns the display name for the theme
    fn name(&self) -> &'static str {
        match self {
            Theme::GitHubLight => "GitHub Light",
            Theme::GitHubDark => "GitHub Dark",
            Theme::GitHubAuto => "GitHub Auto",
        }
    }

    /// Returns all available themes
    fn all() -> &'static [Theme] {
        &[Theme::GitHubLight, Theme::GitHubDark, Theme::GitHubAuto]
    }
}

// Embed the CSS files directly into the binary using include_str!
// Ensure these paths are correct relative to your Cargo.toml or src/main.rs
const GITHUB_LIGHT_CSS: &str = include_str!("../CSS/github-markdown-light.css");
const GITHUB_DARK_CSS: &str = include_str!("../CSS/github-markdown-dark.css");
const GITHUB_AUTO_CSS: &str = include_str!("../CSS/github-markdown-auto.css");


struct App {
    md_path: String,
    pdf_path: String,
    status: String,
    current_theme: Theme, // Store the currently selected theme
    markdown_css: String, // This will hold the currently active CSS
}

impl Default for App {
    fn default() -> Self {
        let mut app = Self {
            md_path: String::new(),
            pdf_path: String::new(),
            status: String::from("Idle"),
            current_theme: Theme::GitHubLight, // Default to light mode
            markdown_css: String::new(), // Will be set by update_active_css
        };
        app.update_active_css(); // Set the initial active CSS
        app
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Markdown to PDF Converter");

            // Markdown file input with "Open..." button
            ui.horizontal(|ui| {
                ui.label("Markdown file:");
                ui.text_edit_singleline(&mut self.md_path);
                if ui.button("Open...").clicked() {
                    if let Some(path) = FileDialog::new()
                        .add_filter("Markdown Files", &["md", "markdown"])
                        .pick_file()
                    {
                        self.md_path = path.to_string_lossy().to_string();
                        // Auto-complete PDF path when MD file is selected
                        self.update_pdf_path_from_md();
                    }
                }
            });

            // Output PDF path (auto-completed)
            ui.horizontal(|ui| {
                ui.label("Output PDF:");
                ui.text_edit_singleline(&mut self.pdf_path);
            });

            // Theme selector
            ui.horizontal(|ui| {
                ui.label("PDF Theme:");
                egui::ComboBox::from_label("")
                    .selected_text(self.current_theme.name())
                    .show_ui(ui, |ui| {
                        for theme in Theme::all() {
                            if ui.selectable_value(&mut self.current_theme, *theme, theme.name()).clicked() {
                                self.update_active_css(); // Update CSS when theme changes
                            }
                        }
                    });
            });


            if ui.button("Convert").clicked() {
                self.convert();
            }

            ui.separator();

            ui.label(format!("Status: {}", self.status));
        });
    }
}

impl App {
    /// Updates the `markdown_css` field based on `current_theme` selection.
    fn update_active_css(&mut self) {
        self.markdown_css = match self.current_theme {
            Theme::GitHubLight => GITHUB_LIGHT_CSS.to_string(),
            Theme::GitHubDark => GITHUB_DARK_CSS.to_string(),
            Theme::GitHubAuto => GITHUB_AUTO_CSS.to_string(),
        };
    }

    /// New method to auto-complete PDF path
    fn update_pdf_path_from_md(&mut self) {
        let md_path_buf = PathBuf::from(&self.md_path);
        if let Some(parent) = md_path_buf.parent() {
            if let Some(stem) = md_path_buf.file_stem() {
                let mut pdf_path_buf = parent.to_path_buf();
                pdf_path_buf.push(stem);
                pdf_path_buf.set_extension("pdf");
                self.pdf_path = pdf_path_buf.to_string_lossy().to_string();
            }
        }
    }

    fn convert(&mut self) {
        if self.md_path.is_empty() || self.pdf_path.is_empty() {
            self.status = "Please fill both paths".to_string();
            return;
        }

        let md_path_buf = PathBuf::from(&self.md_path);
        let pdf_path_buf = PathBuf::from(&self.pdf_path);

        if !md_path_buf.exists() {
            self.status = format!("Error: Markdown file not found at '{}'", self.md_path);
            return;
        }
        if !md_path_buf.is_file() {
            self.status = format!("Error: '{}' is not a file.", self.md_path);
            return;
        }

        match fs::read_to_string(&md_path_buf) {
            Ok(md_text) => {
                let parser = pulldown_cmark::Parser::new(&md_text);
                let mut html_body = String::new();
                pulldown_cmark::html::push_html(&mut html_body, parser);

                let full_html = format!(
                    r#"<!DOCTYPE html>
                    <html>
                    <head>
                        <meta charset="utf-8">
                        <title>Markdown to PDF</title>
                        <style>
                            {}
                        </style>
                    </head>
                    <body>
                        {}
                    </body>
                    </html>"#,
                    // Use the actively selected markdown_css
                    self.markdown_css,
                    html_body
                );

                let temp_dir = std::env::temp_dir();
                let html_file_path = temp_dir.join("temp_markdown_output.html");
                let html_file_str = html_file_path.to_string_lossy().to_string();

                if let Err(e) = fs::write(&html_file_path, full_html) {
                    self.status = format!("Failed to write temporary HTML: {}", e);
                    return;
                }

                if let Some(parent) = pdf_path_buf.parent() {
                    if let Err(e) = fs::create_dir_all(parent) {
                        self.status = format!("Failed to create output directory: {}", e);
                        let _ = fs::remove_file(&html_file_path);
                        return;
                    }
                }

                let output = Command::new("wkhtmltopdf")
                    .arg(&html_file_str)
                    .arg(&pdf_path_buf)
                    .output();

                match output {
                    Ok(command_output) => {
                        if command_output.status.success() {
                            self.status = "Conversion successful!".to_string();
                        } else {
                            let stderr_message = String::from_utf8_lossy(&command_output.stderr);
                            let stdout_message = String::from_utf8_lossy(&command_output.stdout);
                            self.status = format!("Conversion failed. Stderr: {}\nStdout: {}", stderr_message, stdout_message);
                        }
                    }
                    Err(e) => {
                        self.status = format!("Failed to execute wkhtmltopdf. Is it installed and in your PATH? Error: {}", e);
                    }
                }

                let _ = fs::remove_file(&html_file_path);
            }
            Err(e) => {
                self.status = format!("Failed to read Markdown file: {}", e);
            }
        }
    }
}

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        initial_window_size: Some(egui::vec2(500.0, 300.0)), // Set a compact initial window size
        min_window_size: Some(egui::vec2(400.0, 250.0)), // Optional: Set a minimum size
        ..Default::default()
    };
    eframe::run_native(
        "Markdown to PDF Converter",
        options,
        Box::new(|_cc| Box::new(App::default())),
    )
}