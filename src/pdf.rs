use printpdf::*;
use std::fs::File;
use std::io::BufWriter;
use std::path::PathBuf;

use crate::calc::{Alerte, ResultatCalcul, ResultatRapport};
use crate::config::Config;

const TITLE_SIZE: f32 = 16.0;
const SUBTITLE_SIZE: f32 = 11.0;
const TEXT_SIZE: f32 = 9.0;
const TABLE_SIZE: f32 = 8.5;
const SMALL_SIZE: f32 = 7.0;
const LH: f32 = 4.5;       // interligne normal
const LH_TABLE: f32 = 4.0; // interligne tableau
const ML: f32 = 15.0;       // marge gauche
const PW: f32 = 210.0;
const PH: f32 = 297.0;
const MARGIN_BOTTOM: f32 = 15.0;

pub fn generer_pdf(
    r: &ResultatCalcul,
    config: &Config,
    l_par_ha: f64,
    surface_ha: f64,
    dose_produit: f64,
    produit_nom: Option<&str>,
) -> Result<PathBuf, String> {
    let (doc, page1, layer1) = PdfDocument::new(
        "Pulve-Calc - Resultats",
        Mm(PW), Mm(PH), "Page 1",
    );

    let font = doc.add_builtin_font(BuiltinFont::Helvetica).map_err(|e| e.to_string())?;
    let font_bold = doc.add_builtin_font(BuiltinFont::HelveticaBold).map_err(|e| e.to_string())?;

    let layer = doc.get_page(page1).get_layer(layer1);
    let mut y = PH - 12.0;

    // === TITRE ===
    write(&layer, &font_bold, TITLE_SIZE, ML, y, "Pulve-Calc - Feuille de traitement");
    write(&layer, &font, SMALL_SIZE, PW - 45.0, y + 1.0, &chrono_date());
    y -= 6.0;
    hline(&layer, ML, y, PW - ML);
    y -= LH + 2.0;

    // === PARAMETRES (compact: 2 colonnes) ===
    write(&layer, &font_bold, SUBTITLE_SIZE, ML, y, "Parametres");
    y -= LH + 1.0;

    let col2 = 105.0;
    let params = [
        (format!("Volume : {:.0} L/ha", l_par_ha),
         format!("Surface : {:.2} ha", surface_ha)),
        (format!("Dose produit : {:.2} kg ou L/ha", dose_produit),
         format!("Produit : {}", produit_nom.unwrap_or("(non specifie)"))),
        (format!("Largeur : {:.1} m", config.pulverisateur.largeur_travail),
         format!("Citerne : {:.0} L", config.pulverisateur.citerne)),
        (format!("Tracteur : {} ({})", config.tracteur.nom, config.tracteur.moteur),
         format!("PTO : {:.0} - {:.0} t/min", config.tracteur.pto.pto_min, config.tracteur.pto.pto_max)),
        (format!("Pulverisateur : {} - {} x {}", config.pulverisateur.nom, config.pulverisateur.nombre_buses, config.pulverisateur.type_buses),
         String::new()),
    ];
    for (left, right) in &params {
        write(&layer, &font, TEXT_SIZE, ML + 3.0, y, left);
        if !right.is_empty() {
            write(&layer, &font, TEXT_SIZE, col2, y, right);
        }
        y -= LH;
    }
    y -= 2.0;

    // === VOLUMES ET CITERNES ===
    hline(&layer, ML, y, PW - ML);
    y -= LH + 2.0;
    write(&layer, &font_bold, SUBTITLE_SIZE, ML, y, "Volumes et citernes");
    y -= LH + 1.0;

    write(&layer, &font_bold, TEXT_SIZE, ML + 3.0, y,
        &format!("Volume total : {:.0} L  —  {} citernes", r.litres_total, r.nombre_citernes));
    y -= LH + 1.0;

    // Tableau citernes
    let cc = [ML + 3.0, ML + 35.0, ML + 65.0];
    write(&layer, &font_bold, TABLE_SIZE, cc[0], y, "Citerne");
    write(&layer, &font_bold, TABLE_SIZE, cc[1], y, "Eau");
    write(&layer, &font_bold, TABLE_SIZE, cc[2], y, "Produit");
    y -= LH_TABLE;

    for i in 0..r.nombre_citernes {
        let est_derniere = i == r.nombre_citernes - 1;
        let vol = if est_derniere { r.litres_derniere_citerne } else { config.pulverisateur.citerne };
        let prod = if est_derniere { r.produit_derniere } else { r.produit_par_citerne };
        write(&layer, &font, TABLE_SIZE, cc[0], y, &format!("Citerne {}", i + 1));
        write(&layer, &font, TABLE_SIZE, cc[1], y, &format!("{:.0} L", vol));
        write(&layer, &font_bold, TABLE_SIZE, cc[2], y, &format!("{:.2} kg/L", prod));
        y -= LH_TABLE;
    }
    y -= 2.0;

    // === RAPPORTS DE BOITE ===
    hline(&layer, ML, y, PW - ML);
    y -= LH + 2.0;
    write(&layer, &font_bold, SUBTITLE_SIZE, ML, y,
        &format!("Rapports de boite  (regime moteur : {:.0} - {:.0} t/min)", r.regime_moteur_min, r.regime_moteur_max));
    y -= LH + 1.0;

    // Colonnes tableau rapports
    let rc: [f32; 8] = [ML + 1.0, ML + 14.0, ML + 35.0, ML + 56.0, ML + 77.0, ML + 98.0, ML + 119.0, ML + 140.0];
    let headers = ["Rpt", "V min", "V nom", "V max", "P min", "P nom", "P max", "Alertes"];
    let units   = ["",    "(km/h)", "(km/h)", "(km/h)", "(bars)", "(bars)", "(bars)", ""];

    for (j, h) in headers.iter().enumerate() {
        write(&layer, &font_bold, TABLE_SIZE, rc[j], y, h);
    }
    y -= 1.0;
    hline(&layer, ML, y, PW - ML);
    y -= LH_TABLE;

    for (j, u) in units.iter().enumerate() {
        if !u.is_empty() {
            write(&layer, &font, SMALL_SIZE, rc[j], y, u);
        }
    }
    y -= LH_TABLE;

    for rap in &r.rapports {
        // Vérifier si on a besoin d'une nouvelle page
        if y < MARGIN_BOTTOM {
            // On ne peut pas ajouter de page avec printpdf 0.7 facilement sur le même layer
            // mais on peut au moins s'arrêter proprement
            write(&layer, &font, SMALL_SIZE, ML, y, "... suite sur page suivante (non supporte)");
            break;
        }

        let fp = |p: &Option<f64>| match p {
            Some(v) => format!("{:.1}", v),
            None => "-".into(),
        };
        let alertes = format_alertes(rap);

        write(&layer, &font_bold, TABLE_SIZE, rc[0], y, &format!("{}", rap.rapport));
        write(&layer, &font, TABLE_SIZE, rc[1], y, &format!("{:.1}", rap.vitesse_min));
        write(&layer, &font, TABLE_SIZE, rc[2], y, &format!("{:.1}", rap.vitesse_nom));
        write(&layer, &font, TABLE_SIZE, rc[3], y, &format!("{:.1}", rap.vitesse_max));
        write(&layer, &font, TABLE_SIZE, rc[4], y, &fp(&rap.pression_min));
        write(&layer, &font, TABLE_SIZE, rc[5], y, &fp(&rap.pression_nom));
        write(&layer, &font, TABLE_SIZE, rc[6], y, &fp(&rap.pression_max));
        write(&layer, &font, TABLE_SIZE, rc[7], y, &alertes);
        y -= LH_TABLE;
    }

    y -= 3.0;
    hline(&layer, ML, y, PW - ML);
    y -= LH;
    write(&layer, &font, SMALL_SIZE, ML, y,
        "Genere par Pulve-Calc — Les valeurs sont indicatives. Toujours verifier les reglages sur le terrain.");

    // Sauvegarder
    let path = std::env::temp_dir().join("pulve-calc-resultats.pdf");
    let file = File::create(&path).map_err(|e| e.to_string())?;
    doc.save(&mut BufWriter::new(file)).map_err(|e| e.to_string())?;

    Ok(path)
}

fn write(layer: &PdfLayerReference, font: &IndirectFontRef, size: f32, x: f32, y: f32, text: &str) {
    layer.use_text(text, size, Mm(x), Mm(y), font);
}

fn hline(layer: &PdfLayerReference, x1: f32, y: f32, x2: f32) {
    let line = Line {
        points: vec![
            (Point::new(Mm(x1), Mm(y)), false),
            (Point::new(Mm(x2), Mm(y)), false),
        ],
        is_closed: false,
    };
    layer.set_outline_color(Color::Rgb(Rgb::new(0.7, 0.7, 0.7, None)));
    layer.set_outline_thickness(0.3);
    layer.add_line(line);
}

fn format_alertes(rap: &ResultatRapport) -> String {
    if rap.alertes.is_empty() {
        return "OK".into();
    }
    rap.alertes.iter().map(|a| match a {
        Alerte::PressionTropHaute => "P.haute",
        Alerte::PressionTropBasse => "P.basse",
        Alerte::VitesseTropBasse => "V.basse",
        Alerte::VitesseTropHaute => "V.haute",
    }).collect::<Vec<_>>().join(", ")
}

fn chrono_date() -> String {
    let output = std::process::Command::new("date")
        .arg("+%d/%m/%Y %H:%M")
        .output();
    match output {
        Ok(o) => String::from_utf8_lossy(&o.stdout).trim().to_string(),
        Err(_) => String::new(),
    }
}
