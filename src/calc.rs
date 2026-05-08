use crate::config::{Config, Etalonnage};

/// Résultat de calcul pour un rapport de boîte
#[derive(Debug, Clone)]
pub struct ResultatRapport {
    pub rapport: usize,
    pub vitesse_min: f64,       // km/h à PTO max (régime moteur max)
    pub vitesse_nom: f64,       // km/h à PTO nominale
    pub vitesse_max: f64,       // km/h à PTO min (régime moteur min)
    pub debit_nom: f64,         // L/min total à PTO nominale
    pub debit_par_buse_nom: f64,// L/min par buse à PTO nominale
    pub pression_min: Option<f64>,  // bars à vitesse min (PTO max)
    pub pression_nom: Option<f64>,  // bars à vitesse nominale
    pub pression_max: Option<f64>,  // bars à vitesse max (PTO min... wait)
    pub alertes: Vec<Alerte>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Alerte {
    PressionTropHaute,
    PressionTropBasse,
    VitesseTropBasse,
    VitesseTropHaute,
}

/// Résultat global du calcul
#[derive(Debug, Clone)]
pub struct ResultatCalcul {
    pub litres_total: f64,
    pub nombre_citernes: u32,
    pub litres_derniere_citerne: f64,
    pub produit_par_citerne: f64,
    pub produit_derniere: f64,
    pub regime_moteur_min: f64,
    pub regime_moteur_max: f64,
    pub rapports: Vec<ResultatRapport>,
}

/// Calcule le régime moteur pour un régime PTO donné
pub fn regime_moteur_pour_pto(pto_rpm: f64, pto_nominal: f64, moteur_nominal: f64) -> f64 {
    if pto_nominal == 0.0 {
        return 0.0;
    }
    pto_rpm * moteur_nominal / pto_nominal
}

/// Calcule la vitesse au sol pour un rapport à un régime moteur donné
pub fn vitesse_sol(vitesse_max: f64, regime_moteur: f64, regime_max: f64) -> f64 {
    if regime_max == 0.0 {
        return 0.0;
    }
    vitesse_max * regime_moteur / regime_max
}

/// Calcule le débit total requis en L/min
/// Formule : débit = (L/ha × vitesse_km_h × largeur_m) / 600
pub fn debit_requis(l_par_ha: f64, vitesse_km_h: f64, largeur_m: f64) -> f64 {
    (l_par_ha * vitesse_km_h * largeur_m) / 600.0
}

/// Interpole la pression à partir du débit par buse et des points d'étalonnage
/// Retourne None si le débit est hors de la plage d'étalonnage
pub fn pression_pour_debit(debit_buse: f64, etalonnage: &Etalonnage) -> Option<f64> {
    let debits = &etalonnage.debits_par_buse;
    let pressions = &etalonnage.pressions;

    if debits.len() != pressions.len() || debits.is_empty() {
        return None;
    }

    // Hors plage
    if debit_buse < debits[0] || debit_buse > debits[debits.len() - 1] {
        return None;
    }

    // Point exact sur la dernière valeur
    if (debit_buse - debits[debits.len() - 1]).abs() < f64::EPSILON {
        return Some(pressions[pressions.len() - 1]);
    }

    // Recherche de l'intervalle pour interpolation
    for i in 0..debits.len() - 1 {
        if debit_buse >= debits[i] && debit_buse <= debits[i + 1] {
            let denom = debits[i + 1] - debits[i];
            if denom.abs() < f64::EPSILON {
                return Some(pressions[i]);
            }
            let ratio = (debit_buse - debits[i]) / denom;
            let pression = pressions[i] + ratio * (pressions[i + 1] - pressions[i]);
            return Some(pression);
        }
    }

    None
}

fn calcul_pour_regime(
    config: &Config,
    l_par_ha: f64,
    v_max_rapport: f64,
    regime_moteur: f64,
) -> (f64, f64, f64, Option<f64>) {
    let v_sol = vitesse_sol(v_max_rapport, regime_moteur, config.tracteur.regime_max);
    let debit_total = debit_requis(l_par_ha, v_sol, config.pulverisateur.largeur_travail);
    let nombre_buses = config.pulverisateur.nombre_buses.max(1) as f64;
    let debit_buse = debit_total / nombre_buses;
    let pression = pression_pour_debit(debit_buse, &config.pulverisateur.etalonnage);
    (v_sol, debit_total, debit_buse, pression)
}

/// Calcul principal
pub fn calculer(config: &Config, l_par_ha: f64, surface_ha: f64, dose_produit_par_ha: f64) -> ResultatCalcul {
    let citerne = config.pulverisateur.citerne.max(1.0);
    let litres_total = l_par_ha * surface_ha;
    let nombre_citernes = (litres_total / citerne).ceil() as u32;
    let litres_derniere = litres_total - (nombre_citernes.saturating_sub(1) as f64 * citerne);

    let concentration = if l_par_ha > 0.0 {
        dose_produit_par_ha / l_par_ha
    } else {
        0.0
    };
    let produit_par_citerne = concentration * citerne;
    let produit_derniere = concentration * litres_derniere;

    let pto = &config.tracteur.pto;
    let regime_moteur_pto_min = regime_moteur_pour_pto(pto.pto_min, pto.regime_nominal, pto.regime_moteur_nominal);
    let regime_moteur_pto_nom = pto.regime_moteur_nominal;
    let regime_moteur_pto_max = regime_moteur_pour_pto(pto.pto_max, pto.regime_nominal, pto.regime_moteur_nominal);

    let etalonnage = &config.pulverisateur.etalonnage;
    let debit_max_buse = etalonnage.debits_par_buse.last().copied().unwrap_or(f64::MAX);
    let debit_min_buse = etalonnage.debits_par_buse.first().copied().unwrap_or(0.0);

    let mut rapports = Vec::new();
    for (i, &v_max) in config.tracteur.vitesses_max.iter().enumerate() {
        // Calcul aux 3 points PTO : min, nominal, max
        let (v_pto_min, _, _, p_pto_min) = calcul_pour_regime(config, l_par_ha, v_max, regime_moteur_pto_min);
        let (v_nom, debit_nom, debit_buse_nom, p_nom) = calcul_pour_regime(config, l_par_ha, v_max, regime_moteur_pto_nom);
        let (v_pto_max, _, debit_buse_max, p_pto_max) = calcul_pour_regime(config, l_par_ha, v_max, regime_moteur_pto_max);

        let mut alertes = Vec::new();

        // Alertes vitesse (sur la plage complète)
        if v_pto_max < 1.0 {
            alertes.push(Alerte::VitesseTropBasse);
        }
        if v_pto_min > 10.0 {
            alertes.push(Alerte::VitesseTropHaute);
        }

        // Alertes pression (sur la plage complète)
        if debit_buse_max > debit_max_buse {
            alertes.push(Alerte::PressionTropHaute);
        }
        if debit_buse_nom < debit_min_buse {
            alertes.push(Alerte::PressionTropBasse);
        }

        rapports.push(ResultatRapport {
            rapport: i + 1,
            vitesse_min: v_pto_min,
            vitesse_nom: v_nom,
            vitesse_max: v_pto_max,
            debit_nom,
            debit_par_buse_nom: debit_buse_nom,
            pression_min: p_pto_min,
            pression_nom: p_nom,
            pression_max: p_pto_max,
            alertes,
        });
    }

    ResultatCalcul {
        litres_total,
        nombre_citernes,
        litres_derniere_citerne: litres_derniere,
        produit_par_citerne,
        produit_derniere,
        regime_moteur_min: regime_moteur_pto_min,
        regime_moteur_max: regime_moteur_pto_max,
        rapports,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::*;

    fn test_config() -> Config {
        Config {
            tracteur: TracteurConfig {
                nom: "Landini".into(),
                moteur: "Perkins A4-212".into(),
                regime_max: 2200.0,
                vitesses_max: vec![
                    1.302, 2.012, 2.543, 3.186, 3.982, 4.918,
                    6.230, 7.847, 9.735, 12.115, 15.346, 23.978,
                ],
                pto: PtoConfig {
                    regime_nominal: 540.0,
                    regime_moteur_nominal: 1944.0,
                    pto_min: 500.0,
                    pto_max: 560.0,
                },
            },
            pulverisateur: PulverisateurConfig {
                nom: "Atomiseur Ø620".into(),
                nombre_buses: 10,
                type_buses: "Céramique Ø1.0".into(),
                largeur_travail: 4.0,
                citerne: 200.0,
                etalonnage: Etalonnage {
                    pressions: vec![10.0, 15.0, 20.0, 25.0, 30.0, 40.0, 50.0],
                    debits_par_buse: vec![1.88, 2.15, 2.45, 2.72, 2.96, 3.37, 3.70],
                },
            },
        }
    }

    #[test]
    fn test_regime_moteur_pour_pto() {
        let rpm = regime_moteur_pour_pto(540.0, 540.0, 1944.0);
        assert!((rpm - 1944.0).abs() < 0.01);

        let rpm = regime_moteur_pour_pto(500.0, 540.0, 1944.0);
        assert!((rpm - 1800.0).abs() < 0.01);

        // Division par zéro protégée
        let rpm = regime_moteur_pour_pto(540.0, 0.0, 1944.0);
        assert_eq!(rpm, 0.0);
    }

    #[test]
    fn test_vitesse_sol() {
        let v = vitesse_sol(1.302, 1944.0, 2200.0);
        assert!((v - 1.150).abs() < 0.01);

        // Division par zéro protégée
        let v = vitesse_sol(1.302, 1944.0, 0.0);
        assert_eq!(v, 0.0);
    }

    #[test]
    fn test_debit_requis() {
        let d = debit_requis(800.0, 4.0, 4.0);
        assert!((d - 21.33).abs() < 0.1);
    }

    #[test]
    fn test_pression_interpolation() {
        let et = Etalonnage {
            pressions: vec![10.0, 15.0, 20.0, 25.0, 30.0, 40.0, 50.0],
            debits_par_buse: vec![1.88, 2.15, 2.45, 2.72, 2.96, 3.37, 3.70],
        };

        // Débit exactement sur le premier point
        let p = pression_pour_debit(1.88, &et).unwrap();
        assert!((p - 10.0).abs() < 0.01);

        // Débit exactement sur le dernier point
        let p = pression_pour_debit(3.70, &et).unwrap();
        assert!((p - 50.0).abs() < 0.01);

        // Débit interpolé entre 10 et 15 bars
        let p = pression_pour_debit(2.015, &et).unwrap();
        assert!(p > 10.0 && p < 15.0);

        // Hors plage
        assert!(pression_pour_debit(1.0, &et).is_none());
        assert!(pression_pour_debit(4.0, &et).is_none());
    }

    #[test]
    fn test_calcul_citernes() {
        let config = test_config();
        let r = calculer(&config, 800.0, 0.5, 3.6);

        assert_eq!(r.nombre_citernes, 2);
        assert!((r.litres_total - 400.0).abs() < 0.01);
    }

    #[test]
    fn test_produit_par_citerne() {
        let config = test_config();
        let r = calculer(&config, 800.0, 0.5, 3.6);

        // concentration = 3.6/800 = 0.0045 → 0.0045 × 200 = 0.9 kg
        assert!((r.produit_par_citerne - 0.9).abs() < 0.01);
    }

    #[test]
    fn test_rapports_plage_pto() {
        let config = test_config();
        let r = calculer(&config, 800.0, 0.5, 3.6);

        assert_eq!(r.rapports.len(), 12);

        // Chaque rapport a une vitesse min < nom < max (PTO min → régime bas → vitesse basse)
        for rap in &r.rapports {
            assert!(rap.vitesse_min <= rap.vitesse_nom, "rapport {}", rap.rapport);
            assert!(rap.vitesse_nom <= rap.vitesse_max, "rapport {}", rap.rapport);
        }
    }

    #[test]
    fn test_alertes_multiples() {
        let config = test_config();
        let r = calculer(&config, 800.0, 0.5, 3.6);

        // Rapports élevés (10-12) devraient avoir à la fois vitesse trop haute ET pression trop haute
        let r12 = &r.rapports[11];
        assert!(r12.alertes.contains(&Alerte::VitesseTropHaute));
        assert!(r12.alertes.contains(&Alerte::PressionTropHaute));
    }

    #[test]
    fn test_zero_buses_protected() {
        let mut config = test_config();
        config.pulverisateur.nombre_buses = 0;
        let r = calculer(&config, 800.0, 0.5, 3.6);
        // Should not panic, buses clamped to 1
        assert_eq!(r.rapports.len(), 12);
    }
}
