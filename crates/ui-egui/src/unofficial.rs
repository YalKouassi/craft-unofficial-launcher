//! Adding and removing unofficial apps: the Add app dialog with its disclaimer, the empty first-run page, and keeping the cards in step with the catalog.

use std::collections::BTreeMap;

use artcraft_launcher_core::protocol::Command;
use egui::{RichText, Ui};

use crate::app::{Dialog, LauncherUi};
use crate::i18n::fmt;
use crate::model::AppEntry;
use crate::theme::{self, Tokens};
use crate::widgets::{self, Kind};

/// What an app's own releases need so anyone can add it by its repository (the public guide).
const JOINING_URL: &str = "https://github.com/YalKouassi/craft-bridge/blob/main/JOINING.md";

/// The Add app dialog's state, kept while the launcher runs so a long error list survives
/// closing and reopening it.
#[derive(Clone, Debug, Default)]
pub(crate) struct AddState {
    pub(crate) input: String,
    /// The repo being added right now.
    pub(crate) busy: Option<String>,
    /// The last refusal: the repo and every reason.
    pub(crate) errors: Option<(String, Vec<String>)>,
    /// Why each app's newest descriptor was refused, by app id.
    pub(crate) problems: BTreeMap<String, Vec<String>>,
}

impl LauncherUi {
    /// The catalog changed: keep each card that's still listed (with its state), add new ones and
    /// drop the removed.
    pub(crate) fn on_catalog_changed(&mut self, problems: BTreeMap<String, Vec<String>>) {
        let mut old = std::mem::take(&mut self.entries);
        self.entries = artcraft_launcher_core::apps()
            .iter()
            .map(|app| match old.iter().position(|e| e.app.id == app.id) {
                Some(i) => {
                    let mut e = old.swap_remove(i);
                    e.app = app;
                    e
                }
                None => AppEntry::new(app),
            })
            .collect();
        self.add.problems = problems;
        if self.detail.as_deref().is_some_and(|id| self.entry(id).is_none()) {
            self.detail = None;
        }
    }

    pub(crate) fn on_add_finished(&mut self, repo: String, result: Result<String, Vec<String>>) {
        if self.add.busy.as_deref() == Some(repo.as_str()) {
            self.add.busy = None;
        }
        match result {
            Ok(id) => {
                let name = self.entry(&id).map_or(id.clone(), |e| e.app.name.to_string());
                self.toast(fmt(tl!("Added {name}"), &[("name", &name)]), false);
                self.add.errors = None;
                self.add.input.clear();
                if matches!(self.dialog, Some(Dialog::AddApp)) {
                    self.dialog = None;
                }
                // The new app's install and release aren't known yet.
                self.send(Command::Detect);
                self.send(Command::CheckForUpdates { force: true });
            }
            Err(problems) => self.add.errors = Some((repo, problems)),
        }
    }

    /// Opens the Add app dialog (and fetches the index the first time).
    pub(crate) fn open_add_dialog(&mut self) {
        self.dialog = Some(Dialog::AddApp);
    }

    fn start_add(&mut self, repo: String) {
        let repo = repo.trim().to_string();
        if repo.is_empty() || self.add.busy.is_some() {
            return;
        }
        self.add.errors = None;
        self.add.busy = Some(repo.clone());
        self.act_command(Command::AddApp { repo });
    }

    /// The toolbar's Add app button.
    pub(crate) fn add_button(&mut self, ui: &mut Ui) {
        if widgets::button(ui, tl!("Add app"), Kind::Secondary, 0.0).on_hover_text(tl!("Add an unofficial app by its GitHub repository")).clicked() {
            self.open_add_dialog();
        }
    }

    /// What the page shows before any app is added.
    pub(crate) fn empty_catalog(&mut self, ui: &mut Ui) {
        let t = Tokens::get(ui.ctx());
        ui.add_space(48.0);
        ui.vertical_centered(|ui| {
            ui.set_max_width(520.0);
            ui.label(widgets::heading(tl!("No apps yet"), theme::DIALOG_TITLE, t.ink_strong));
            ui.add_space(10.0);
            ui.add(
                egui::Label::new(
                    RichText::new(tl!(
                        "Add unofficial Craft-style apps by their GitHub repository. Each app describes itself with a craft-app.json in its releases; the launcher checks it, then installs and updates the app like any other."
                    ))
                    .color(t.muted),
                )
                .wrap(),
            );
            ui.add_space(18.0);
            if widgets::button(ui, tl!("Add app"), Kind::Primary, 140.0).clicked() {
                self.open_add_dialog();
            }
        });
    }

    /// The disclaimer every unofficial app carries.
    pub(crate) fn unofficial_notice(ui: &mut Ui) {
        let t = Tokens::get(ui.ctx());
        egui::Frame::new().stroke(egui::Stroke::new(1.0, t.line_strong)).inner_margin(10).show(ui, |ui| {
            widgets::hud(ui, tl!("Unofficial apps"), t.bad);
            ui.add_space(4.0);
            ui.add(
                egui::Label::new(
                    RichText::new(tl!(
                        "Not created, verified, monitored, or supported by ArtCraft or by this launcher's maintainers. They accept no liability for the code, security, stability, or behavior of this software. Send bugs, security issues and support requests to the app's own repository."
                    ))
                    .color(t.muted),
                )
                .wrap(),
            );
        });
    }

    /// The Add app dialog. Returns true to close it.
    pub(crate) fn add_app_dialog(&mut self, ui: &mut Ui) -> bool {
        let t = Tokens::get(ui.ctx());
        ui.set_width(520.0f32.min(ui.ctx().content_rect().width() - 80.0));
        ui.label(widgets::heading(tl!("Add an unofficial app"), theme::DIALOG_TITLE, t.ink_strong));
        ui.add_space(10.0);
        Self::unofficial_notice(ui);
        ui.add_space(12.0);
        widgets::hud(ui, tl!("GitHub repository"), t.faint);
        ui.add_space(4.0);
        let busy = self.add.busy.clone();
        let mut submit = false;
        ui.horizontal(|ui| {
            let edit = ui.add_enabled(
                busy.is_none(),
                egui::TextEdit::singleline(&mut self.add.input)
                    .hint_text("owner/name")
                    .desired_width(ui.available_width() - 110.0)
                    .min_size(egui::vec2(0.0, theme::CONTROL_H))
                    .margin(egui::vec2(10.0, 0.0)),
            );
            submit = edit.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
            if widgets::button(ui, if busy.is_some() { tl!("Checking…") } else { tl!("Add") }, Kind::Primary, 100.0).clicked() && busy.is_none() {
                submit = true;
            }
        });
        if submit {
            let repo = self.add.input.clone();
            self.start_add(repo);
        }
        if let Some((repo, problems)) = &self.add.errors {
            ui.add_space(10.0);
            ui.label(RichText::new(fmt(tl!("{repo} wasn't added:"), &[("repo", repo)])).color(t.bad));
            egui::ScrollArea::vertical().id_salt("add_errors").max_height(140.0).show(ui, |ui| {
                for p in problems {
                    ui.add(egui::Label::new(RichText::new(format!("• {p}")).color(t.muted)).wrap());
                }
            });
        }
        // For app authors: how to make their own repo something anyone can add here.
        ui.add_space(14.0);
        ui.add(
            egui::Label::new(
                RichText::new(tl!("Got a repo? Follow these guidelines, then anyone can add your app here by entering its GitHub URL."))
                    .color(t.muted),
            )
            .wrap(),
        );
        ui.add_space(4.0);
        widgets::hud_link(ui, tl!("Guidelines for app authors"), JOINING_URL);
        ui.add_space(14.0);
        let mut close = false;
        ui.horizontal(|ui| {
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                close = widgets::button(ui, tl!("Close"), Kind::Secondary, 90.0).clicked();
            });
        });
        close
    }

    /// The card menu's entry that takes an app off the list, and any refusal of its newest
    /// descriptor.
    pub(crate) fn menu_remove(&mut self, ui: &mut Ui, id: &str) {
        let t = Tokens::get(ui.ctx());
        if let Some(problems) = self.add.problems.get(id) {
            ui.separator();
            ui.label(RichText::new(tl!("Newest release refused")).color(t.bad));
            for p in problems {
                ui.add(egui::Label::new(RichText::new(p).color(t.muted).small()).wrap());
            }
        }
        ui.separator();
        if ui.button(tl!("Remove from list")).on_hover_text(tl!("The app stays installed; only the launcher forgets it")).clicked() {
            self.act_command(Command::RemoveApp { app: id.to_string() });
            ui.close();
        }
    }
}
