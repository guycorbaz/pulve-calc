use eframe::egui;
use crate::calc::{self, Alerte, ResultatCalcul};
use crate::config::Config;
use crate::db::{Database, Produit, DoseCulture};
use crate::pdf;

/// Parse un nombre en supportant la virgule décimale française
fn parse_f64_fr(s: &str) -> Result<f64, ()> {
    let normalized = s.replace(',', ".");
    normalized.parse::<f64>().map_err(|_| ())
}

/// Dessine un cadre avec titre
fn frame_with_title(ui: &mut egui::Ui, title: &str, add_contents: impl FnOnce(&mut egui::Ui)) {
    ui.add_space(4.0);
    egui::Frame::group(ui.style())
        .inner_margin(egui::Margin::same(12))
        .corner_radius(egui::CornerRadius::same(6))
        .show(ui, |ui| {
            ui.strong(title);
            ui.add_space(6.0);
            add_contents(ui);
        });
    ui.add_space(4.0);
}

pub struct PulveApp {
    config: Config,
    db: Database,

    // Entrées utilisateur
    l_par_ha: String,
    surface_ha: String,
    dose_produit: String,
    largeur_travail: String,
    citerne: String,

    // Résultats
    resultat: Option<ResultatCalcul>,

    // Paramètres mémorisés pour le PDF
    derniers_params: Option<ParamsCalcul>,

    // Produits
    produits: Vec<Produit>,
    selected_produit: Option<usize>,

    // Onglet actif
    onglet: Onglet,

    // Nouveau produit
    nouveau_produit: ProduitForm,

    // Config éditables
    pto_min: String,
    pto_max: String,

    // Messages
    message: Option<(String, bool)>,
}

#[derive(Clone)]
pub struct ParamsCalcul {
    pub l_par_ha: f64,
    pub surface_ha: f64,
    pub dose_produit: f64,
    pub largeur_travail: f64,
    pub citerne: f64,
    pub produit_nom: Option<String>,
}

#[derive(PartialEq)]
enum Onglet {
    Calcul,
    Resultats,
    Produits,
    Configuration,
}

struct ProduitForm {
    nom: String,
    type_produit: String,
    composition: String,
    culture: String,
    dose: String,
    concentration: String,
    notes: String,
}

impl Default for ProduitForm {
    fn default() -> Self {
        Self {
            nom: String::new(),
            type_produit: String::new(),
            composition: String::new(),
            culture: String::new(),
            dose: String::new(),
            concentration: String::new(),
            notes: String::new(),
        }
    }
}

impl PulveApp {
    pub fn new(config: Config, db: Database) -> Self {
        let produits = db.list_produits().unwrap_or_default();
        let largeur_travail = config.pulverisateur.largeur_travail.to_string();
        let citerne = config.pulverisateur.citerne.to_string();
        let pto_min = config.tracteur.pto.pto_min.to_string();
        let pto_max = config.tracteur.pto.pto_max.to_string();
        Self {
            config,
            db,
            l_par_ha: "800".into(),
            surface_ha: "0.5".into(),
            dose_produit: "3.6".into(),
            largeur_travail,
            citerne,
            resultat: None,
            derniers_params: None,
            produits,
            selected_produit: None,
            onglet: Onglet::Calcul,
            nouveau_produit: ProduitForm::default(),
            pto_min,
            pto_max,
            message: None,
        }
    }

    fn set_error(&mut self, msg: impl Into<String>) {
        self.message = Some((msg.into(), true));
    }

    fn set_info(&mut self, msg: impl Into<String>) {
        self.message = Some((msg.into(), false));
    }

    pub fn set_config_warning(&mut self, msg: String) {
        self.message = Some((msg, true));
    }

    fn calculer(&mut self) {
        let l_ha = match parse_f64_fr(&self.l_par_ha) {
            Ok(v) if v > 0.0 => v,
            _ => { self.set_error("Volume L/ha invalide"); return; }
        };
        let surface = match parse_f64_fr(&self.surface_ha) {
            Ok(v) if v > 0.0 => v,
            _ => { self.set_error("Surface invalide"); return; }
        };
        let dose = match parse_f64_fr(&self.dose_produit) {
            Ok(v) if v >= 0.0 => v,
            _ => { self.set_error("Dose produit invalide"); return; }
        };
        match parse_f64_fr(&self.largeur_travail) {
            Ok(v) if v > 0.0 => self.config.pulverisateur.largeur_travail = v,
            _ => { self.set_error("Largeur inter-rangs invalide"); return; }
        }
        match parse_f64_fr(&self.citerne) {
            Ok(v) if v > 0.0 => self.config.pulverisateur.citerne = v,
            _ => { self.set_error("Volume citerne invalide"); return; }
        }
        match parse_f64_fr(&self.pto_min) {
            Ok(v) if v > 0.0 => self.config.tracteur.pto.pto_min = v,
            _ => { self.set_error("PTO min invalide"); return; }
        }
        match parse_f64_fr(&self.pto_max) {
            Ok(v) if v > 0.0 => self.config.tracteur.pto.pto_max = v,
            _ => { self.set_error("PTO max invalide"); return; }
        }

        let produit_nom = self.selected_produit
            .and_then(|i| self.produits.get(i))
            .map(|p| p.nom.clone());

        self.derniers_params = Some(ParamsCalcul {
            l_par_ha: l_ha,
            surface_ha: surface,
            dose_produit: dose,
            largeur_travail: self.config.pulverisateur.largeur_travail,
            citerne: self.config.pulverisateur.citerne,
            produit_nom,
        });

        self.resultat = Some(calc::calculer(&self.config, l_ha, surface, dose));
        self.message = None;
        self.onglet = Onglet::Resultats;
    }

    fn imprimer_pdf(&mut self) {
        let Some(r) = &self.resultat else { return };
        let Some(params) = &self.derniers_params else { return };

        match pdf::generer_pdf(
            r,
            &self.config,
            params.l_par_ha,
            params.surface_ha,
            params.dose_produit,
            params.produit_nom.as_deref(),
        ) {
            Ok(path) => {
                if let Err(e) = open::that(&path) {
                    self.set_error(format!("Impossible d'ouvrir le PDF: {e}"));
                } else {
                    self.set_info(format!("PDF généré: {}", path.display()));
                }
            }
            Err(e) => self.set_error(format!("Erreur génération PDF: {e}")),
        }
    }

    // ==================== ONGLET CALCUL ====================

    fn ui_calcul(&mut self, ui: &mut egui::Ui) {
        frame_with_title(ui, "Paramètres de traitement", |ui| {
            egui::Grid::new("params_grid")
                .num_columns(4)
                .spacing([16.0, 6.0])
                .show(ui, |ui| {
                    ui.label("Volume (L/ha) :");
                    ui.add(egui::TextEdit::singleline(&mut self.l_par_ha).desired_width(80.0));
                    ui.label("Surface (ha) :");
                    ui.add(egui::TextEdit::singleline(&mut self.surface_ha).desired_width(80.0));
                    ui.end_row();

                    ui.label("Dose produit (kg ou L/ha) :");
                    ui.add(egui::TextEdit::singleline(&mut self.dose_produit).desired_width(80.0));
                    ui.label("Largeur inter-rangs (m) :");
                    ui.add(egui::TextEdit::singleline(&mut self.largeur_travail).desired_width(80.0));
                    ui.end_row();

                    ui.label("Citerne (L) :");
                    ui.add(egui::TextEdit::singleline(&mut self.citerne).desired_width(80.0));
                    ui.label("");
                    ui.label("");
                    ui.end_row();
                });
        });

        frame_with_title(ui, "Prise de force (PTO)", |ui| {
            ui.horizontal(|ui| {
                ui.label("PTO min :");
                ui.add(egui::TextEdit::singleline(&mut self.pto_min).desired_width(60.0));
                ui.label("t/min");
                ui.add_space(20.0);
                ui.label("PTO max :");
                ui.add(egui::TextEdit::singleline(&mut self.pto_max).desired_width(60.0));
                ui.label("t/min");
            });
        });

        if !self.produits.is_empty() {
            frame_with_title(ui, "Produit", |ui| {
                egui::ComboBox::from_id_salt("produit_select")
                    .selected_text(
                        self.selected_produit
                            .and_then(|i| self.produits.get(i))
                            .map(|p| p.nom.as_str())
                            .unwrap_or("-- Sélectionner --"),
                    )
                    .width(250.0)
                    .show_ui(ui, |ui| {
                        if ui.selectable_label(self.selected_produit.is_none(), "-- Aucun --").clicked() {
                            self.selected_produit = None;
                        }
                        for (i, p) in self.produits.iter().enumerate() {
                            if ui.selectable_label(self.selected_produit == Some(i), &p.nom).clicked() {
                                self.selected_produit = Some(i);
                                if let Some(dose) = p.doses.first() {
                                    self.dose_produit = dose.dose_kg_ha.to_string();
                                }
                            }
                        }
                    });
            });
        }

        ui.add_space(8.0);
        if ui.button("  Calculer  ").clicked() {
            self.calculer();
        }

        if let Some((msg, is_error)) = &self.message {
            ui.add_space(4.0);
            let color = if *is_error { egui::Color32::RED } else { egui::Color32::GREEN };
            ui.colored_label(color, msg);
        }
    }

    // ==================== ONGLET RÉSULTATS ====================

    fn ui_resultats(&mut self, ui: &mut egui::Ui) {
        let Some(r) = self.resultat.clone() else {
            ui.heading("Aucun résultat");
            ui.label("Allez dans l'onglet Calcul pour lancer un calcul.");
            return;
        };

        // Boutons en haut
        ui.horizontal(|ui| {
            if ui.button("  Imprimer (PDF)  ").clicked() {
                self.imprimer_pdf();
            }
            if ui.button("  Nouveau calcul  ").clicked() {
                self.onglet = Onglet::Calcul;
            }
        });

        if let Some((msg, is_error)) = &self.message {
            let color = if *is_error { egui::Color32::RED } else { egui::Color32::GREEN };
            ui.colored_label(color, msg);
        }

        ui.add_space(4.0);

        // --- Résumé paramètres ---
        if let Some(params) = &self.derniers_params {
            frame_with_title(ui, "Paramètres du traitement", |ui| {
                ui.horizontal_wrapped(|ui| {
                    ui.label(format!("{:.0} L/ha", params.l_par_ha));
                    ui.weak("|");
                    ui.label(format!("{:.2} ha", params.surface_ha));
                    ui.weak("|");
                    ui.label(format!("Dose: {:.2} kg-L/ha", params.dose_produit));
                    ui.weak("|");
                    ui.label(format!("Largeur: {:.1} m", params.largeur_travail));
                    ui.weak("|");
                    ui.label(format!("Citerne: {:.0} L", params.citerne));
                    if let Some(nom) = &params.produit_nom {
                        ui.weak("|");
                        ui.strong(nom);
                    }
                });
            });
        }

        // --- Volumes et citernes ---
        frame_with_title(ui, "Volumes et citernes", |ui| {
            egui::Grid::new("volumes_grid")
                .num_columns(4)
                .spacing([20.0, 6.0])
                .show(ui, |ui| {
                    ui.label("Volume total :");
                    ui.heading(format!("{:.0} L", r.litres_total));
                    ui.label("Nombre de citernes :");
                    ui.heading(format!("{}", r.nombre_citernes));
                    ui.end_row();
                });
        });

        // --- Quantité produit par citerne ---
        if r.produit_par_citerne > 0.0 {
            egui::Frame::group(ui.style())
                .inner_margin(egui::Margin::same(12))
                .corner_radius(egui::CornerRadius::same(6))
                .fill(egui::Color32::from_rgb(40, 60, 40))
                .show(ui, |ui| {
                    ui.strong("Quantité de produit par citerne");
                    ui.separator();

                    egui::Grid::new("produit_detail_grid")
                        .num_columns(3)
                        .spacing([20.0, 6.0])
                        .show(ui, |ui| {
                            for i in 0..r.nombre_citernes {
                                let est_derniere = i == r.nombre_citernes - 1;
                                let vol = if est_derniere { r.litres_derniere_citerne } else { self.config.pulverisateur.citerne };
                                let prod = if est_derniere { r.produit_derniere } else { r.produit_par_citerne };
                                ui.strong(format!("Citerne {}", i + 1));
                                ui.label(format!("{:.0} L d'eau", vol));
                                ui.heading(format!("{:.2} kg/L de produit", prod));
                                ui.end_row();
                            }
                        });
                });
            ui.add_space(4.0);
        }

        // --- Rapports de boîte ---
        frame_with_title(ui, "Rapports de boîte", |ui| {
            ui.label(format!("Régime moteur : {:.0} - {:.0} t/min (PTO {:.0} - {:.0} t/min)",
                r.regime_moteur_min, r.regime_moteur_max,
                self.config.tracteur.pto.pto_min, self.config.tracteur.pto.pto_max));
            ui.add_space(4.0);

            egui::ScrollArea::vertical()
                .max_height(400.0)
                .show(ui, |ui| {
                    egui::Grid::new("rapports_grid")
                        .num_columns(8)
                        .spacing([10.0, 4.0])
                        .striped(true)
                        .min_col_width(55.0)
                        .show(ui, |ui| {
                            for h in &["Rapport", "V min", "V nom", "V max", "P min", "P nom", "P max", "Alertes"] {
                                ui.strong(*h);
                            }
                            ui.end_row();

                            for h in &["", "(km/h)", "(km/h)", "(km/h)", "(bars)", "(bars)", "(bars)", ""] {
                                ui.weak(*h);
                            }
                            ui.end_row();

                            for rap in &r.rapports {
                                let has_alerts = !rap.alertes.is_empty();
                                let color = if has_alerts {
                                    egui::Color32::from_rgb(130, 130, 130)
                                } else {
                                    egui::Color32::from_rgb(220, 220, 220)
                                };

                                ui.colored_label(color, format!("{}", rap.rapport));
                                ui.colored_label(color, format!("{:.1}", rap.vitesse_min));
                                ui.colored_label(color, format!("{:.1}", rap.vitesse_nom));
                                ui.colored_label(color, format!("{:.1}", rap.vitesse_max));

                                let fmt_p = |p: &Option<f64>| match p {
                                    Some(v) => format!("{:.1}", v),
                                    None => "-".into(),
                                };
                                ui.colored_label(color, fmt_p(&rap.pression_min));
                                ui.colored_label(color, fmt_p(&rap.pression_nom));
                                ui.colored_label(color, fmt_p(&rap.pression_max));

                                if has_alerts {
                                    let txt: Vec<&str> = rap.alertes.iter().map(|a| match a {
                                        Alerte::PressionTropHaute => "P. haute",
                                        Alerte::PressionTropBasse => "P. basse",
                                        Alerte::VitesseTropBasse => "V. basse",
                                        Alerte::VitesseTropHaute => "V. haute",
                                    }).collect();
                                    ui.colored_label(egui::Color32::YELLOW, txt.join(", "));
                                } else {
                                    ui.colored_label(egui::Color32::GREEN, "OK");
                                }
                                ui.end_row();
                            }
                        });
                });
        });
    }

    // ==================== ONGLET PRODUITS ====================

    fn ui_produits(&mut self, ui: &mut egui::Ui) {
        frame_with_title(ui, "Produits enregistrés", |ui| {
            if self.produits.is_empty() {
                ui.label("Aucun produit enregistré.");
            } else {
                let mut to_delete: Option<usize> = None;
                for (i, p) in self.produits.iter().enumerate() {
                    ui.horizontal(|ui| {
                        ui.strong(&p.nom);
                        if !p.type_produit.is_empty() {
                            ui.label(format!("({})", p.type_produit));
                        }
                        if !p.composition.is_empty() {
                            ui.weak(format!("- {}", p.composition));
                        }
                        if ui.small_button("Supprimer").clicked() {
                            to_delete = Some(i);
                        }
                    });
                    for dose in &p.doses {
                        ui.indent(format!("dose_{i}"), |ui| {
                            ui.label(format!(
                                "  {}: {:.1} kg/ha ({:.2}%) {}",
                                dose.culture, dose.dose_kg_ha, dose.concentration_pct, dose.notes
                            ));
                        });
                    }
                    ui.separator();
                }
                if let Some(idx) = to_delete {
                    if let Some(p) = self.produits.get(idx) {
                        if let Some(id) = p.id {
                            let _ = self.db.delete_produit(id);
                        }
                    }
                    self.produits.remove(idx);
                }
            }
        });

        frame_with_title(ui, "Ajouter un produit", |ui| {
            let form = &mut self.nouveau_produit;
            egui::Grid::new("new_product_grid")
                .num_columns(4)
                .spacing([16.0, 6.0])
                .show(ui, |ui| {
                    ui.label("Nom :");
                    ui.add(egui::TextEdit::singleline(&mut form.nom).desired_width(280.0));
                    ui.label("Type :");
                    ui.add(egui::TextEdit::singleline(&mut form.type_produit).desired_width(280.0));
                    ui.end_row();

                    ui.label("Composition :");
                    ui.add(egui::TextEdit::singleline(&mut form.composition).desired_width(280.0));
                    ui.label("Culture :");
                    ui.add(egui::TextEdit::singleline(&mut form.culture).desired_width(280.0));
                    ui.end_row();

                    ui.label("Dose (kg/ha) :");
                    ui.add(egui::TextEdit::singleline(&mut form.dose).desired_width(280.0));
                    ui.label("Concentration (%) :");
                    ui.add(egui::TextEdit::singleline(&mut form.concentration).desired_width(280.0));
                    ui.end_row();
                });

            ui.add_space(6.0);
            ui.label("Notes :");
            ui.add(
                egui::TextEdit::multiline(&mut form.notes)
                    .desired_width(640.0)
                    .desired_rows(3),
            );

            ui.add_space(8.0);
            if ui.button("Ajouter").clicked() {
                if form.nom.is_empty() {
                    self.message = Some(("Nom du produit requis".into(), true));
                    return;
                }
                let dose_val = match parse_f64_fr(&form.dose) {
                    Ok(v) => v,
                    Err(_) => {
                        self.set_error("Dose invalide (utilisez . ou , pour les décimales)");
                        return;
                    }
                };
                let conc_val = parse_f64_fr(&form.concentration).unwrap_or(0.0);

                let dose = DoseCulture {
                    culture: form.culture.clone(),
                    dose_kg_ha: dose_val,
                    concentration_pct: conc_val,
                    notes: form.notes.clone(),
                };
                let produit = Produit {
                    id: None,
                    nom: form.nom.clone(),
                    type_produit: form.type_produit.clone(),
                    composition: form.composition.clone(),
                    doses: vec![dose],
                };
                match self.db.insert_produit(&produit) {
                    Ok(_) => {
                        self.produits = self.db.list_produits().unwrap_or_default();
                        self.nouveau_produit = ProduitForm::default();
                        self.set_info("Produit ajouté");
                    }
                    Err(e) => self.set_error(format!("Erreur: {e}")),
                }
            }
        });

        if let Some((msg, is_error)) = &self.message {
            ui.add_space(4.0);
            let color = if *is_error { egui::Color32::RED } else { egui::Color32::GREEN };
            ui.colored_label(color, msg);
        }
    }

    // ==================== ONGLET CONFIGURATION ====================

    fn ui_configuration(&mut self, ui: &mut egui::Ui) {
        frame_with_title(ui, "Tracteur", |ui| {
            ui.horizontal(|ui| {
                ui.label(format!("{} - {}", self.config.tracteur.nom, self.config.tracteur.moteur));
                ui.weak(format!("(régime max: {:.0} t/min)", self.config.tracteur.regime_max));
            });

            ui.add_space(4.0);
            ui.label("Vitesses par rapport (km/h au régime max) :");
            ui.horizontal_wrapped(|ui| {
                for (i, v) in self.config.tracteur.vitesses_max.iter().enumerate() {
                    ui.label(format!("{}:{:.1}", i + 1, v));
                    if i < self.config.tracteur.vitesses_max.len() - 1 {
                        ui.weak("|");
                    }
                }
            });

            ui.add_space(6.0);
            ui.label(format!("PTO nominale : {:.0} t/min a {:.0} t/min moteur",
                self.config.tracteur.pto.regime_nominal,
                self.config.tracteur.pto.regime_moteur_nominal));

            egui::Grid::new("pto_edit_grid")
                .num_columns(4)
                .spacing([10.0, 4.0])
                .show(ui, |ui| {
                    ui.label("PTO min :");
                    ui.add(egui::TextEdit::singleline(&mut self.pto_min).desired_width(60.0));
                    ui.label("PTO max :");
                    ui.add(egui::TextEdit::singleline(&mut self.pto_max).desired_width(60.0));
                    ui.end_row();
                });
        });

        frame_with_title(ui, "Pulvérisateur", |ui| {
            ui.label(format!("{} - {} x {}",
                self.config.pulverisateur.nom,
                self.config.pulverisateur.nombre_buses,
                self.config.pulverisateur.type_buses));

            ui.add_space(4.0);
            egui::Grid::new("pulve_edit_grid")
                .num_columns(4)
                .spacing([10.0, 4.0])
                .show(ui, |ui| {
                    ui.label("Largeur travail (m) :");
                    ui.add(egui::TextEdit::singleline(&mut self.largeur_travail).desired_width(60.0));
                    ui.label("Citerne (L) :");
                    ui.add(egui::TextEdit::singleline(&mut self.citerne).desired_width(60.0));
                    ui.end_row();
                });

            ui.add_space(6.0);
            ui.label("Étalonnage :");
            egui::Grid::new("etal_grid")
                .num_columns(self.config.pulverisateur.etalonnage.pressions.len() + 1)
                .spacing([8.0, 2.0])
                .show(ui, |ui| {
                    ui.strong("Pression (bars)");
                    for p in &self.config.pulverisateur.etalonnage.pressions {
                        ui.label(format!("{:.0}", p));
                    }
                    ui.end_row();
                    ui.strong("Débit/buse (L/min)");
                    for d in &self.config.pulverisateur.etalonnage.debits_par_buse {
                        ui.label(format!("{:.2}", d));
                    }
                    ui.end_row();
                });
        });

        ui.add_space(8.0);
        if ui.button("  Sauvegarder la configuration  ").clicked() {
            if let Ok(v) = parse_f64_fr(&self.pto_min) { self.config.tracteur.pto.pto_min = v; }
            if let Ok(v) = parse_f64_fr(&self.pto_max) { self.config.tracteur.pto.pto_max = v; }
            if let Ok(v) = parse_f64_fr(&self.largeur_travail) { self.config.pulverisateur.largeur_travail = v; }
            if let Ok(v) = parse_f64_fr(&self.citerne) { self.config.pulverisateur.citerne = v; }

            match self.config.save() {
                Ok(()) => self.set_info("Configuration sauvegardée"),
                Err(e) => self.set_error(format!("Erreur: {e}")),
            }
        }

        if let Some((msg, is_error)) = &self.message {
            ui.add_space(4.0);
            let color = if *is_error { egui::Color32::RED } else { egui::Color32::GREEN };
            ui.colored_label(color, msg);
        }
    }
}

impl eframe::App for PulveApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::TopBottomPanel::top("tabs").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.selectable_value(&mut self.onglet, Onglet::Calcul, "  Calcul  ");
                let resultats_label = if self.resultat.is_some() { "  Résultats  " } else { "  Résultats  " };
                ui.selectable_value(&mut self.onglet, Onglet::Resultats, resultats_label);
                ui.selectable_value(&mut self.onglet, Onglet::Produits, "  Produits  ");
                ui.selectable_value(&mut self.onglet, Onglet::Configuration, "  Configuration  ");
            });
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                match self.onglet {
                    Onglet::Calcul => self.ui_calcul(ui),
                    Onglet::Resultats => self.ui_resultats(ui),
                    Onglet::Produits => self.ui_produits(ui),
                    Onglet::Configuration => self.ui_configuration(ui),
                }
            });
        });
    }
}
