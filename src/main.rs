use eframe::{egui, epi};
use std::process::Command;
use std::fs;
use std::path::PathBuf; // For more robust path handling

struct App {
    md_path: String,
    pdf_path: String,
    status: String,
    // Add a field to store the CSS content
    markdown_css: String,
}

impl Default for App {
    fn default() -> Self {
        let mut app = Self {
            md_path: String::new(),
            pdf_path: String::new(),
            status: String::from("Idle"),
            markdown_css: String::new(), // Initialize as empty
        };
        // Attempt to load the CSS on startup
        app.load_default_css();
        app
    }
}

impl epi::App for App {
    fn name(&self) -> &str {
        "Markdown to PDF Converter"
    }

    fn update(&mut self, ctx: &egui::Context, _frame: &epi::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Markdown to PDF Converter");

            ui.horizontal(|ui| {
                ui.label("Markdown file:");
                ui.text_edit_singleline(&mut self.md_path);
            });

            ui.horizontal(|ui| {
                ui.label("Output PDF:");
                ui.text_edit_singleline(&mut self.pdf_path);
            });

            if ui.button("Convert").clicked() {
                self.convert();
            }

            ui.separator(); // Add a visual separator

            // Option to load custom CSS
            ui.horizontal(|ui| {
                ui.label("Custom CSS file (optional):");
                if ui.button("Load CSS").clicked() {
                    // In a real app, you'd open a file dialog here.
                    // For this example, let's assume a 'styles.css' in the same directory.
                    match fs::read_to_string("styles.css") {
                        Ok(css) => {
                            self.markdown_css = css;
                            self.status = "Custom CSS loaded.".to_string();
                        },
                        Err(e) => {
                            self.status = format!("Failed to load custom CSS: {}", e);
                        }
                    }
                }
            });

            ui.label(format!("Status: {}", self.status));
        });
    }
}

impl App {
    fn load_default_css(&mut self) {
        // This is where you would ideally load a default, bundled CSS file
        // that closely mimics VS Code's Markdown preview styling.
        // For demonstration, a very basic CSS is provided.
        // REPLACE THIS WITH YOUR EXTRACTED VS CODE CSS.
        self.markdown_css = r#"
            body {
                font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, Helvetica, Arial, sans-serif;
                line-height: 1.6;
                margin: 20px;
                color: #333; /* Darker text */
            }
            h1, h2, h3, h4, h5, h6 {
                color: #007acc; /* VS Code blue for headings */
                border-bottom: 1px solid #eee;
                padding-bottom: 0.3em;
                margin-top: 1em;
            }
            code {
                font-family: 'SFMono-Regular', Consolas, 'Liberation Mono', Menlo, Courier, monospace;
                background-color: rgba(10, 10, 10, 0.05); /* Light gray for inline code */
                padding: 0.2em 0.4em;
                border-radius: 3px;
            }
            pre code {
                display: block;
                overflow-x: auto;
                background-color: #f6f8fa; /* GitHub-like background for code blocks */
                padding: 16px;
                border-radius: 6px;
                line-height: 1.45;
                color: #333;
            }
            blockquote {
                border-left: 0.25em solid #dfe2e5;
                color: #6a737d;
                padding: 0 1em;
                margin-left: 0;
            }
            table {
                border-collapse: collapse;
                width: 100%;
            }
            th, td {
                border: 1px solid #ddd;
                padding: 8px;
                text-align: left;
            }
            th {
                background-color: #f2f2f2;
            }
            a {
                color: #007acc; /* VS Code link color */
                text-decoration: none;
            }
            a:hover {
                text-decoration: underline;
            }
        "#.to_string();
    }

    fn convert(&mut self) {
        if self.md_path.is_empty() || self.pdf_path.is_empty() {
            self.status = "Please fill both paths".to_string();
            return;
        }

        // Use PathBuf for better OS compatibility
        let md_path_buf = PathBuf::from(&self.md_path);
        let pdf_path_buf = PathBuf::from(&self.pdf_path);

        match fs::read_to_string(&md_path_buf) {
            Ok(md_text) => {
                let parser = pulldown_cmark::Parser::new(&md_text);
                let mut html_body = String::new();
                pulldown_cmark::html::push_html(&mut html_body, parser);

                // Construct the full HTML with embedded CSS
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
                    self.markdown_css, // Your loaded CSS goes here
                    html_body
                );

                let html_file = "temp.html"; // Temporary HTML file
                if let Err(e) = fs::write(html_file, full_html) {
                    self.status = format!("Failed to write temporary HTML: {}", e);
                    return;
                }

                let output = Command::new("wkhtmltopdf")
                    .arg(html_file)
                    .arg(&pdf_path_buf) // Use PathBuf for output
                    .output();

                match output {
                    Ok(command_output) => {
                        if command_output.status.success() {
                            self.status = "Conversion successful!".to_string();
                        } else {
                            // Capture stderr for more detailed error messages
                            let stderr_message = String::from_utf8_lossy(&command_output.stderr);
                            self.status = format!("Conversion failed: {}", stderr_message);
                        }
                    }
                    Err(e) => {
                        self.status = format!("Failed to execute wkhtmltopdf: {}", e);
                    }
                }

                // Clean up the temporary HTML file
                let _ = fs::remove_file(html_file);
            }
            Err(e) => {
                self.status = format!("Failed to read Markdown file: {}", e);
            }
        }
    }
}

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions::default();
    eframe::run_native(
        "Markdown to PDF Converter",
        options,
        Box::new(|_cc| Box::new(App::default())),
    )
}