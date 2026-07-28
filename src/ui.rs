use crate::launcher;
use crate::model::{AppConfig, Project, Resource, ResourceType, Template};
use crate::config;
use egui::{
    Align, Align2, Color32, CornerRadius, FontId, Frame, Id, Layout, Margin, RichText, Sense,
    Stroke, TextEdit, Vec2,
};
use std::time::{Duration, Instant};

pub const PROJECT_COLORS: [&str; 8] = [
    "#6366f1", "#0ea5e9", "#10b981", "#f59e0b", "#ef4444", "#ec4899", "#8b5cf6", "#14b8a6",
];

fn hex_color(hex: &str) -> Color32 {
    let h = hex.trim_start_matches('#');
    if h.len() == 6 {
        if let Ok(v) = u32::from_str_radix(h, 16) {
            return Color32::from_rgb((v >> 16) as u8, (v >> 8) as u8, v as u8);
        }
    }
    Color32::from_rgb(0x63, 0x66, 0xf1)
}

struct Toast {
    message: String,
    is_error: bool,
    created: Instant,
}

/// Modal state: editing a copy of a project (new or existing).
struct Editor {
    draft: Project,
    is_new: bool,
    error: String,
    template_name: String,
}

impl Editor {
    fn open(draft: Project, is_new: bool) -> Self {
        Editor {
            draft,
            is_new,
            error: String::new(),
            template_name: String::new(),
        }
    }
}

pub struct LaunchXApp {
    cfg: AppConfig,
    query: String,
    editor: Option<Editor>,
    toasts: Vec<Toast>,
}

impl LaunchXApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        cc.egui_ctx.set_visuals(egui::Visuals::dark());
        let cjk_loaded = crate::fonts::install_cjk_fallback(&cc.egui_ctx).is_some();
        let (cfg, mut toasts) = match config::load() {
            Ok(c) => (c, Vec::new()),
            Err(e) => (
                AppConfig::default(),
                vec![Toast {
                    message: format!("Failed to load config: {e}"),
                    is_error: true,
                    created: Instant::now(),
                }],
            ),
        };
        if !cjk_loaded {
            toasts.push(Toast {
                message: "No CJK system font found — Chinese text may not display.".into(),
                is_error: true,
                created: Instant::now(),
            });
        }
        LaunchXApp {
            cfg,
            query: String::new(),
            editor: None,
            toasts,
        }
    }

    fn toast(&mut self, message: impl Into<String>, is_error: bool) {
        self.toasts.push(Toast {
            message: message.into(),
            is_error,
            created: Instant::now(),
        });
    }

    fn persist(&mut self) {
        if let Err(e) = config::save(&self.cfg) {
            self.toast(format!("Failed to save config: {e}"), true);
        }
    }

    fn launch(&mut self, idx: usize) {
        let project = &self.cfg.projects[idx];
        if project.resources.is_empty() {
            let msg = format!("\"{}\" has no resources configured.", project.name);
            self.toast(msg, true);
            return;
        }
        let results = launcher::launch_all(&project.resources);
        let name = project.name.clone();
        let failed: Vec<_> = results.iter().filter(|r| r.error.is_some()).collect();
        if failed.is_empty() {
            let msg = format!("Launched {} resource(s) for \"{}\"", results.len(), name);
            self.toast(msg, false);
        } else {
            let msgs: Vec<String> = failed
                .iter()
                .map(|f| format!("{}: {}", f.name, f.error.as_deref().unwrap_or("")))
                .collect();
            for m in msgs {
                self.toast(m, true);
            }
        }
    }

    // ---------- UI sections ----------

    fn header(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label(RichText::new("⚡ LaunchX").font(FontId::proportional(20.0)).strong());
            ui.add_space(12.0);
            ui.add(
                TextEdit::singleline(&mut self.query)
                    .hint_text("Search projects…")
                    .desired_width(260.0),
            );
            // "×" (U+00D7) is covered by egui's built-in font; "✕" rendered as tofu.
            if !self.query.is_empty() && ui.small_button("×").clicked() {
                self.query.clear();
            }
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if ui
                    .button(RichText::new("＋ New Project").strong())
                    .clicked()
                {
                    self.editor = Some(Editor::open(Project::new(), true));
                }
            });
        });
    }

    fn grid(&mut self, ui: &mut egui::Ui) {
        let q = self.query.trim().to_lowercase();
        let visible: Vec<usize> = self
            .cfg
            .projects
            .iter()
            .enumerate()
            .filter(|(_, p)| {
                q.is_empty()
                    || p.name.to_lowercase().contains(&q)
                    || p.resources.iter().any(|r| {
                        r.name.to_lowercase().contains(&q)
                            || r.target.to_lowercase().contains(&q)
                    })
            })
            .map(|(i, _)| i)
            .collect();

        if visible.is_empty() {
            ui.add_space(80.0);
            ui.vertical_centered(|ui| {
                ui.label(RichText::new("No projects found").size(18.0).weak());
                ui.label(RichText::new("Click “＋ New Project” to create one.").weak());
            });
            return;
        }

        let card_w = 260.0;
        let cols = ((ui.available_width() + 12.0) / (card_w + 12.0)).floor().max(1.0) as usize;

        let mut launch_idx: Option<usize> = None;
        let mut edit_idx: Option<usize> = None;

        egui::ScrollArea::vertical().show(ui, |ui| {
            for row in visible.chunks(cols) {
                // Top-align so cards in a row don't shift vertically.
                ui.horizontal_top(|ui| {
                    for &idx in row {
                        let p = &self.cfg.projects[idx];
                        let (launch, edit) = project_card(ui, p, card_w);
                        if launch {
                            launch_idx = Some(idx);
                        }
                        if edit {
                            edit_idx = Some(idx);
                        }
                    }
                });
                ui.add_space(12.0);
            }
        });

        if let Some(idx) = launch_idx {
            self.launch(idx);
        }
        if let Some(idx) = edit_idx {
            self.editor = Some(Editor::open(self.cfg.projects[idx].clone(), false));
        }
    }

    fn editor_modal(&mut self, ctx: &egui::Context) {
        // Cloned so the window closure doesn't fight the &mut self.editor borrow.
        let templates = self.cfg.templates.clone();
        let Some(editor) = &mut self.editor else { return };
        let mut close = false;
        let mut save = false;
        let mut delete = false;
        let mut apply_template: Option<usize> = None;
        let mut save_template = false;
        let mut delete_template: Option<usize> = None;

        let title = if editor.is_new { "New Project" } else { "Edit Project" };
        egui::Window::new(title)
            .id(Id::new("editor"))
            .collapsible(false)
            .resizable(false)
            .anchor(Align2::CENTER_CENTER, Vec2::ZERO)
            .default_width(560.0)
            .show(ctx, |ui| {
                ui.add_space(4.0);

                // Name
                ui.label("Name");
                ui.add(
                    TextEdit::singleline(&mut editor.draft.name)
                        .hint_text("My Project")
                        .desired_width(f32::INFINITY),
                );
                ui.add_space(8.0);

                // Color picker
                ui.label("Color");
                ui.horizontal(|ui| {
                    for c in PROJECT_COLORS {
                        let selected = editor.draft.color == c;
                        let (rect, resp) =
                            ui.allocate_exact_size(Vec2::splat(20.0), Sense::click());
                        let painter = ui.painter();
                        painter.circle_filled(rect.center(), 8.0, hex_color(c));
                        if selected {
                            painter.circle_stroke(
                                rect.center(),
                                10.0,
                                Stroke::new(2.0_f32, Color32::WHITE),
                            );
                        }
                        if resp.clicked() {
                            editor.draft.color = c.to_string();
                        }
                    }
                });
                ui.add_space(8.0);

                // Templates: load a saved resource set, or save the current one.
                ui.horizontal(|ui| {
                    ui.label("Template");
                    egui::ComboBox::from_id_salt(Id::new("tpl_pick"))
                        .width(160.0)
                        .selected_text("Load template…")
                        .show_ui(ui, |ui| {
                            if templates.is_empty() {
                                ui.label(RichText::new("No templates saved").weak());
                            }
                            for (i, t) in templates.iter().enumerate() {
                                ui.horizontal(|ui| {
                                    if ui
                                        .selectable_label(
                                            false,
                                            format!("{} ({})", t.name, t.resources.len()),
                                        )
                                        .clicked()
                                    {
                                        apply_template = Some(i);
                                    }
                                    if ui.small_button("🗑").on_hover_text("Delete template").clicked() {
                                        delete_template = Some(i);
                                    }
                                });
                            }
                        });
                    ui.add(
                        TextEdit::singleline(&mut editor.template_name)
                            .hint_text("Template name")
                            .desired_width(140.0),
                    );
                    if ui.button("Save as template").clicked() {
                        save_template = true;
                    }
                });
                ui.add_space(8.0);

                // Resources
                ui.horizontal(|ui| {
                    ui.label("Resources");
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        for t in ResourceType::ALL.iter().rev() {
                            if ui
                                .small_button(format!("＋ {} {}", t.glyph(), t.label()))
                                .clicked()
                            {
                                editor.draft.resources.push(Resource::new(*t));
                            }
                        }
                    });
                });
                ui.add_space(4.0);

                if editor.draft.resources.is_empty() {
                    ui.label(
                        RichText::new("No resources yet — add a URL, file, or folder above.")
                            .weak(),
                    );
                }

                let mut remove: Option<usize> = None;
                for (i, r) in editor.draft.resources.iter_mut().enumerate() {
                    ui.horizontal(|ui| {
                        ui.label(r.kind.glyph());
                        ui.add(
                            TextEdit::singleline(&mut r.name)
                                .hint_text("Label")
                                .desired_width(110.0),
                        );
                        let target_w = if r.kind == ResourceType::Folder { 210.0 } else { 300.0 };
                        ui.add(
                            TextEdit::singleline(&mut r.target)
                                .hint_text(r.kind.placeholder())
                                .desired_width(target_w),
                        );
                        // Native file/folder picker.
                        match r.kind {
                            ResourceType::File => {
                                if ui.button("…").on_hover_text("Browse for file").clicked() {
                                    if let Some(p) = rfd::FileDialog::new().pick_file() {
                                        r.target = p.to_string_lossy().into_owned();
                                    }
                                }
                            }
                            ResourceType::Folder => {
                                if ui.button("…").on_hover_text("Browse for folder").clicked() {
                                    if let Some(p) = rfd::FileDialog::new().pick_folder() {
                                        r.target = p.to_string_lossy().into_owned();
                                    }
                                }
                            }
                            ResourceType::Url => {}
                        }
                        if r.kind == ResourceType::Folder {
                            let mut in_code = r.open_with.as_deref() == Some("code");
                            egui::ComboBox::from_id_salt(Id::new(("openwith", i)))
                                .width(90.0)
                                .selected_text(if in_code { "VS Code" } else { "Explorer" })
                                .show_ui(ui, |ui| {
                                    ui.selectable_value(&mut in_code, false, "Explorer");
                                    ui.selectable_value(&mut in_code, true, "VS Code");
                                });
                            r.open_with = in_code.then(|| "code".to_string());
                        }
                        if ui.small_button("🗑").clicked() {
                            remove = Some(i);
                        }
                    });
                }
                if let Some(i) = remove {
                    editor.draft.resources.remove(i);
                }

                if !editor.error.is_empty() {
                    ui.add_space(6.0);
                    ui.colored_label(Color32::from_rgb(0xef, 0x44, 0x44), &editor.error);
                }

                ui.add_space(10.0);
                ui.separator();
                ui.horizontal(|ui| {
                    if !editor.is_new
                        && ui
                            .button(RichText::new("Delete project").color(Color32::from_rgb(0xef, 0x44, 0x44)))
                            .clicked()
                    {
                        delete = true;
                    }
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if ui.button(RichText::new("Save").strong()).clicked() {
                            save = true;
                        }
                        if ui.button("Cancel").clicked() {
                            close = true;
                        }
                    });
                });
            });

        if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            close = true;
        }

        if let Some(i) = apply_template {
            let editor = self.editor.as_mut().unwrap();
            let tpl = &self.cfg.templates[i];
            // Merge, skipping resources already present (same type + target).
            let mut added = 0;
            let mut skipped = 0;
            for res in &tpl.resources {
                let dup = editor.draft.resources.iter().any(|r| {
                    r.kind == res.kind
                        && r.target.trim().eq_ignore_ascii_case(res.target.trim())
                });
                if dup {
                    skipped += 1;
                } else {
                    let mut r = res.clone();
                    r.id = uuid::Uuid::new_v4().to_string();
                    editor.draft.resources.push(r);
                    added += 1;
                }
            }
            let msg = if skipped > 0 {
                format!("Template applied: {added} added, {skipped} duplicate(s) skipped")
            } else {
                format!("Template applied: {added} resource(s) added")
            };
            self.toast(msg, false);
        } else if save_template {
            let editor = self.editor.as_mut().unwrap();
            let name = editor.template_name.trim().to_string();
            if name.is_empty() {
                editor.error = "Template name is required.".into();
            } else if editor.draft.resources.is_empty() {
                editor.error = "Add at least one resource before saving a template.".into();
            } else {
                let resources = editor.draft.resources.clone();
                editor.template_name.clear();
                editor.error.clear();
                // Overwrite an existing template with the same name.
                if let Some(t) = self.cfg.templates.iter_mut().find(|t| t.name == name) {
                    t.resources = resources;
                } else {
                    self.cfg.templates.push(Template {
                        id: uuid::Uuid::new_v4().to_string(),
                        name: name.clone(),
                        resources,
                    });
                }
                self.persist();
                self.toast(format!("Template \"{name}\" saved"), false);
            }
        } else if let Some(i) = delete_template {
            let name = self.cfg.templates.remove(i).name;
            self.persist();
            self.toast(format!("Template \"{name}\" deleted"), false);
        }

        if save {
            let editor = self.editor.as_mut().unwrap();
            editor.error = validate(&editor.draft);
            if editor.error.is_empty() {
                let mut draft = editor.draft.clone();
                draft.name = draft.name.trim().to_string();
                for r in &mut draft.resources {
                    r.target = r.target.trim().to_string();
                    if r.name.trim().is_empty() {
                        r.name = r.target.clone();
                    } else {
                        r.name = r.name.trim().to_string();
                    }
                }
                let is_new = editor.is_new;
                if is_new {
                    self.cfg.projects.push(draft);
                } else if let Some(p) =
                    self.cfg.projects.iter_mut().find(|p| p.id == draft.id)
                {
                    *p = draft;
                }
                self.persist();
                self.editor = None;
            }
        } else if delete {
            let id = self.editor.as_ref().unwrap().draft.id.clone();
            self.cfg.projects.retain(|p| p.id != id);
            self.persist();
            self.editor = None;
        } else if close {
            self.editor = None;
        }
    }

    fn toasts_overlay(&mut self, ctx: &egui::Context) {
        const SUCCESS_VISIBLE_SECS: f32 = 4.0;
        const WARNING_VISIBLE_SECS: f32 = 5.0;
        const FADE_SECS: f32 = 0.4;

        self.toasts.retain(|t| {
            let visible_secs = if t.is_error {
                WARNING_VISIBLE_SECS
            } else {
                SUCCESS_VISIBLE_SECS
            };
            t.created.elapsed().as_secs_f32() < visible_secs + FADE_SECS
        });
        if self.toasts.is_empty() {
            return;
        }

        egui::Area::new(Id::new("toasts"))
            .anchor(Align2::RIGHT_BOTTOM, Vec2::new(-16.0, -16.0))
            .show(ctx, |ui| {
                for t in &self.toasts {
                    let age = t.created.elapsed().as_secs_f32();
                    let visible_secs = if t.is_error {
                        WARNING_VISIBLE_SECS
                    } else {
                        SUCCESS_VISIBLE_SECS
                    };
                    let opacity = if age <= visible_secs {
                        1.0
                    } else {
                        (1.0 - (age - visible_secs) / FADE_SECS).clamp(0.0, 1.0)
                    };
                    let bg = if t.is_error {
                        Color32::from_rgb(0xdc, 0x26, 0x26)
                    } else {
                        Color32::from_rgb(0x05, 0x96, 0x69)
                    }
                    .gamma_multiply(opacity);
                    Frame::new()
                        .fill(bg)
                        .corner_radius(CornerRadius::same(6))
                        .inner_margin(Margin::symmetric(12, 8))
                        .show(ui, |ui| {
                            ui.label(
                                RichText::new(&t.message)
                                    .color(Color32::WHITE.gamma_multiply(opacity)),
                            );
                        });
                    ui.add_space(6.0);
                }
            });

        // Repaint smoothly while a toast is fading; otherwise checking a few
        // times per second is enough until its five-second warning period ends.
        let fading = self.toasts.iter().any(|t| {
            let visible_secs = if t.is_error {
                WARNING_VISIBLE_SECS
            } else {
                SUCCESS_VISIBLE_SECS
            };
            t.created.elapsed().as_secs_f32() > visible_secs
        });
        ctx.request_repaint_after(if fading {
            Duration::from_millis(16)
        } else {
            Duration::from_millis(250)
        });
    }
}

fn validate(p: &Project) -> String {
    if p.name.trim().is_empty() {
        return "Project name is required.".into();
    }
    for r in &p.resources {
        let target = r.target.trim();
        if target.is_empty() {
            return "Every resource needs a target (URL or path).".into();
        }
        if r.kind == ResourceType::Url
            && !(target.starts_with("http://") || target.starts_with("https://"))
        {
            return format!("\"{target}\" must start with http:// or https://");
        }
    }
    String::new()
}

/// Draws one card. Returns (launch_requested, edit_requested).
fn project_card(ui: &mut egui::Ui, p: &Project, width: f32) -> (bool, bool) {
    let mut launch = false;
    let mut edit = false;
    let mut edit_button_rect: Option<egui::Rect> = None;
    let accent = hex_color(&p.color);

    let frame = Frame::new()
        .fill(ui.visuals().extreme_bg_color)
        .stroke(Stroke::new(1.0_f32, ui.visuals().widgets.noninteractive.bg_stroke.color))
        .corner_radius(CornerRadius::same(10))
        .inner_margin(Margin::same(12));

    let resp = frame
        .show(ui, |ui| {
            ui.set_width(width);
            // Fixed height so every card is uniform regardless of content.
            ui.set_height(44.0);
            ui.horizontal(|ui| {
                // Colored icon chip
                let (rect, _) = ui.allocate_exact_size(Vec2::splat(36.0), Sense::hover());
                ui.painter().rect_filled(
                    rect,
                    CornerRadius::same(8),
                    accent.gamma_multiply(0.25),
                );
                ui.painter().text(
                    rect.center(),
                    Align2::CENTER_CENTER,
                    "⚡",
                    FontId::proportional(18.0),
                    accent,
                );

                ui.vertical(|ui| {
                    ui.label(RichText::new(&p.name).strong());
                    let (urls, files, folders) = p.counts();
                    let mut parts = Vec::new();
                    if urls > 0 {
                        parts.push(format!("🌐 {urls} URL{}", if urls > 1 { "s" } else { "" }));
                    }
                    if files > 0 {
                        parts.push(format!("📄 {files} File{}", if files > 1 { "s" } else { "" }));
                    }
                    if folders > 0 {
                        parts.push(format!("📁 {folders} Folder{}", if folders > 1 { "s" } else { "" }));
                    }
                    if parts.is_empty() {
                        ui.label(RichText::new("No resources").weak().size(11.0));
                    } else {
                        ui.label(RichText::new(parts.join("  ")).weak().size(11.0));
                    }
                });

                ui.with_layout(Layout::right_to_left(Align::Min), |ui| {
                    let response = ui.small_button("✏").on_hover_text("Edit project");
                    edit_button_rect = Some(response.rect);
                    if response.clicked() {
                        edit = true;
                    }
                });
            });
        })
        .response;

    // Keep the card's launch interaction from overlapping the edit button. An
    // interaction registered over the whole frame here would sit on top of the
    // button and consume its clicks.
    let mut launch_rect = resp.rect;
    if let Some(edit_rect) = edit_button_rect {
        launch_rect.max.x = (edit_rect.min.x - ui.spacing().item_spacing.x)
            .max(launch_rect.min.x);
    }
    let launch_resp = ui.interact(launch_rect, resp.id.with("launch"), Sense::click());
    if launch_resp.double_clicked() {
        launch = true;
    }
    launch_resp.on_hover_text("Double-click to launch · use the edit icon to edit");

    (launch, edit)
}

impl eframe::App for LaunchXApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::TopBottomPanel::top("header")
            .frame(Frame::new().inner_margin(Margin::symmetric(16, 10)).fill(ctx.style().visuals.panel_fill))
            .show(ctx, |ui| self.header(ui));

        egui::CentralPanel::default()
            .frame(Frame::new().inner_margin(Margin::same(16)).fill(ctx.style().visuals.panel_fill))
            .show(ctx, |ui| self.grid(ui));

        self.editor_modal(ctx);
        self.toasts_overlay(ctx);
    }
}
