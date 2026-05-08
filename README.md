# Pulve-Calc

Calculateur de pulvérisation pour atomiseur viticole / arboricole monté sur tracteur agricole. L'outil détermine, à partir d'une dose à l'hectare et d'une surface, les **rapports de boîte utilisables**, les **régimes PTO**, les **vitesses au sol** et les **pressions** correspondantes — avec alertes lorsque les paramètres sortent des plages d'étalonnage.

L'application est **agnostique du matériel** : tracteur (régime moteur, vitesses par rapport, plage PTO), pulvérisateur (nombre de buses, largeur de travail, capacité citerne) et **étalonnage des buses** (table pression / débit) sont entièrement paramétrables. Le fichier livré (`config/default.toml`) contient des valeurs d'exemple à adapter à votre matériel.

> Statut : projet personnel, fonctionnel. Interface en français. Testé sous Linux.

## Fonctionnalités

- **Calcul** : entrer la dose visée (L/ha), la surface (ha) et la dose de produit (kg/ha) → obtenir pour chaque rapport de boîte la fenêtre de vitesses, débits et pressions exploitables.
- **Citerne** : nombre de remplissages nécessaires, volume du dernier remplissage, quantité de produit à mettre par citerne.
- **Catalogue de produits** : enregistrer noms, compositions, doses recommandées par culture, notes — stockés dans une base SQLite locale.
- **Configuration éditable** : tracteur (régime max, vitesses par rapport, plage PTO), pulvérisateur (largeur, nombre de buses, capacité citerne, table d'étalonnage pression/débit).
- **Export PDF** du rapport de calcul (récapitulatif + tableau par rapport).
- **Alertes** automatiques : pression trop basse / trop haute, vitesse trop basse / trop haute.

## Aperçu de l'interface

L'application présente quatre onglets :

| Onglet | Rôle |
|---|---|
| **Calcul** | Saisie des paramètres, choix éventuel d'un produit du catalogue, lancement du calcul. |
| **Résultats** | Récapitulatif citernes + tableau ligne-par-rapport (vitesses min/nom/max, débits, pressions, alertes), avec impression PDF. |
| **Produits** | Catalogue persistant de produits phytosanitaires et de leurs doses. |
| **Configuration** | Édition de la fiche tracteur, plage PTO, paramètres pulvérisateur et étalonnage. |

## Prérequis

- **Rust** 1.75+ (édition 2021) — installer via [rustup](https://rustup.rs/).
- **Linux** : dépendances système nécessaires à `eframe`/`egui` :
  ```bash
  # Debian / Ubuntu
  sudo apt install libxcb1-dev libxrandr-dev libxi-dev libgl1-mesa-dev \
                   libxkbcommon-dev libwayland-dev pkg-config
  ```
  L'application devrait également compiler sur macOS et Windows (non testé).

## Installation et compilation

```bash
git clone https://github.com/guycorbaz/pulve-calc.git
cd pulve-calc
cargo run --release
```

Les tests unitaires (formules de calcul, interpolation pression, gestion des bornes) :

```bash
cargo test
```

## Configuration par défaut

Le fichier [`config/default.toml`](config/default.toml) contient des **valeurs d'exemple** chargées au premier démarrage. Elles décrivent un tracteur Landini équipé d'un atomiseur Ø620 mm — il s'agit d'un point de départ à **adapter à votre matériel réel** (régime max, vitesses par rapport, plage PTO, étalonnage des buses, capacité citerne…) avant toute utilisation. La configuration peut être modifiée directement dans le fichier ou via l'onglet **Configuration** de l'application.

```toml
[tracteur]
nom = "Landini"
moteur = "Perkins A4-212"
regime_max = 2200          # t/min
vitesses_max = [
    1.302, 2.012, 2.543, 3.186, 3.982, 4.918,
    6.230, 7.847, 9.735, 12.115, 15.346, 23.978
]                          # km/h, par rapport, au régime max

[tracteur.pto]
regime_nominal = 540
regime_moteur_nominal = 1944
pto_min = 500
pto_max = 560

[pulverisateur]
nom = "Atomiseur Ø620"
nombre_buses = 10
type_buses = "Céramique Ø1.0"
largeur_travail = 4.0      # m (inter-rangs)
citerne = 200              # litres

[pulverisateur.etalonnage]
pressions       = [10.0, 15.0, 20.0, 25.0, 30.0, 40.0, 50.0]
debits_par_buse = [1.88, 2.15, 2.45, 2.72, 2.96, 3.37, 3.70]   # L/min
```

Voir [`docs/USAGE.md`](docs/USAGE.md) pour une description détaillée de chaque champ.

## Stockage des données utilisateur

L'application écrit dans les emplacements standards de la plateforme (via la crate [`dirs`](https://crates.io/crates/dirs)) :

| Donnée | Linux (XDG) | macOS | Windows |
|---|---|---|---|
| Configuration utilisateur | `~/.config/pulve-calc/config.toml` | `~/Library/Application Support/pulve-calc/` | `%APPDATA%\pulve-calc\` |
| Base produits SQLite | `~/.local/share/pulve-calc/pulve.db` | `~/Library/Application Support/pulve-calc/` | `%APPDATA%\pulve-calc\` |

Aucune donnée n'est envoyée à l'extérieur.

## Méthodologie de calcul

Les formules clés sont :

- **Régime moteur pour un PTO donné** :
  `rpm_moteur = pto_rpm × (rpm_moteur_nominal / pto_nominal)`
- **Vitesse au sol** sur un rapport :
  `v_sol = v_max_rapport × (rpm_moteur / rpm_max)`
- **Débit total requis** :
  `débit_L/min = (L/ha × v_km/h × largeur_m) / 600`
- **Pression** : interpolation linéaire dans la table d'étalonnage à partir du débit par buse.

Pour chaque rapport de boîte, le calcul est réalisé aux **trois points** de la plage PTO (min, nominal, max) afin de présenter une **fenêtre exploitable** plutôt qu'une valeur unique.

Détails et exemples : [`docs/USAGE.md`](docs/USAGE.md).

## Architecture du code

```
src/
├── main.rs       # Point d'entrée + style egui
├── ui.rs         # Interface (onglets Calcul/Résultats/Produits/Configuration)
├── calc.rs       # Formules + structures de résultat (testé)
├── config.rs     # Chargement / sauvegarde de la config TOML
├── db.rs         # Couche SQLite pour le catalogue produits
└── pdf.rs        # Génération du rapport PDF
```

Dépendances principales : [`eframe`](https://crates.io/crates/eframe) (interface), [`rusqlite`](https://crates.io/crates/rusqlite) (catalogue), [`printpdf`](https://crates.io/crates/printpdf) (export), [`serde`](https://crates.io/crates/serde) + [`toml`](https://crates.io/crates/toml) (configuration).

## Limitations connues

- Interface en français uniquement.
- Étalonnage interpolé linéairement entre points fournis ; pas d'extrapolation hors plage (l'application le signale par une alerte).
- Le calcul suppose la table de vitesses au régime maximal — la mise à jour des rapports doit refléter votre tracteur réel.
- Aucune intégration GPS / DPAE — l'outil est un assistant de réglage avant chantier.

## Contribuer

Les bugs et suggestions sont bienvenus via les *issues* GitHub. Si vous adaptez l'outil pour un autre matériel (autre tracteur, autre atomiseur), une PR enrichissant `config/` avec un fichier d'exemple est très volontiers acceptée.

Avant de soumettre une PR :

```bash
cargo fmt
cargo clippy --all-targets -- -D warnings
cargo test
```

## Licence

[MIT](LICENSE) — © 2026 Guy Corbaz.

Les manuels manufacturier (Landini, Airone, Braxol) référencés pendant le développement **ne sont pas redistribués** dans ce dépôt pour des raisons de droits d'auteur.
