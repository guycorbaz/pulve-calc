# Guide d'utilisation — Pulve-Calc

Ce guide détaille les onglets de l'application, les formules utilisées et les bonnes pratiques pour calibrer un atomiseur.

---

## 1. Démarrage

Lancer l'application :

```bash
cargo run --release
```

Au premier démarrage, le fichier `config/default.toml` du dépôt est utilisé pour initialiser la configuration utilisateur dans `~/.config/pulve-calc/config.toml` (Linux). Toute modification dans l'onglet **Configuration** est sauvegardée à cet emplacement et réutilisée aux lancements suivants.

---

## 2. Onglet **Calcul**

### Champs à saisir

| Champ | Unité | Rôle |
|---|---|---|
| Dose à l'hectare | L/ha | Volume de bouillie souhaité par hectare |
| Surface | ha | Surface totale à traiter |
| Dose de produit | kg/ha | Masse de matière active / produit commercial par hectare |
| Produit (optionnel) | — | Pré-remplit la dose de produit depuis le catalogue |

### Bouton **Calculer**

Calcule :

- le **volume total de bouillie** ;
- le **nombre de citernes** nécessaires (capacité définie dans la configuration) ;
- le **volume du dernier remplissage** (≤ capacité citerne) ;
- la **quantité de produit** à mettre par citerne (et pour le dernier remplissage, qui est souvent partiel) ;
- pour **chaque rapport de boîte** :
  - vitesse au sol aux trois points de la plage PTO (min / nominal / max),
  - débit total et débit par buse (au régime nominal),
  - pression interpolée correspondante,
  - alertes éventuelles.

L'onglet **Résultats** est automatiquement sélectionné après le calcul.

---

## 3. Onglet **Résultats**

Affiche le récapitulatif citernes en haut, puis un **tableau ligne-par-rapport** :

| Colonne | Sens |
|---|---|
| Rapport | Numéro du rapport de boîte (1 = le plus court) |
| V min / V nom / V max | km/h, à PTO max / nominale / min |
| Débit (nom) | L/min total au régime nominal |
| Débit/buse (nom) | L/min par buse au régime nominal |
| P min / P nom / P max | bars (interpolés depuis l'étalonnage) |
| Statut | OK ou liste d'alertes |

### Alertes

| Alerte | Cause |
|---|---|
| **P. basse** | Débit par buse au régime nominal en dessous du premier point d'étalonnage |
| **P. haute** | Débit par buse au régime PTO max au-dessus du dernier point d'étalonnage |
| **V. basse** | Vitesse au sol < 1 km/h sur toute la plage PTO (rapport trop court) |
| **V. haute** | Vitesse au sol > 10 km/h sur toute la plage PTO (rapport trop long) |

Un rapport peut cumuler plusieurs alertes (typiquement les rapports les plus longs : *V. haute + P. haute*).

### Impression PDF

Le bouton **Imprimer (PDF)** génère un fichier dans un dossier temporaire et le pousse vers le visualiseur système. Le rapport contient le récapitulatif citernes et le tableau complet.

---

## 4. Onglet **Produits**

Catalogue persistant (SQLite). Pour chaque produit :

- **Nom**, **Type** (fongicide, insecticide, etc.), **Composition** (matières actives)
- une ou plusieurs entrées **dose / culture / concentration / notes**

L'utilisation classique est :

1. Saisir une fois les produits utilisés (Airone, Braxol, etc.) avec leurs doses constructeur par culture ;
2. Dans l'onglet **Calcul**, sélectionner le produit → la dose kg/ha est pré-remplie ;
3. Lancer le calcul.

La base est stockée dans `~/.local/share/pulve-calc/pulve.db` (Linux). Pour repartir de zéro, supprimer ce fichier.

---

## 5. Onglet **Configuration**

### Tracteur

- **Nom / Moteur** : libellés pour information.
- **Régime max** : régime moteur correspondant à la table des vitesses ci-dessous.
- **Vitesses par rapport** : km/h **au régime max**. Mesurées au sol (pneus de série), elles sont la base de toute extrapolation à un régime moteur donné.
- **PTO nominale / régime moteur nominal** : couple de référence — par exemple 540 t/min PTO à 1944 t/min moteur.
- **PTO min / PTO max** : bornes de la plage exploitable. Définissent les colonnes V min / V max et P min / P max.

### Pulvérisateur

- **Nombre de buses** : intervient directement dans le débit par buse.
- **Largeur de travail** : inter-rangs effectif en mètres. Pour adapter à un autre inter-rang, conserver la table d'étalonnage et changer ce champ — ne **pas** retoucher l'étalonnage.
- **Capacité citerne** : litres.
- **Étalonnage** : couples (pression, débit par buse). Doit être croissant. Le calcul interpole linéairement entre points adjacents et **n'extrapole pas** hors plage.

---

## 6. Méthodologie de calcul

### 6.1 Régime moteur correspondant à un régime PTO

```
rpm_moteur = pto_rpm × (rpm_moteur_nominal / pto_nominal)
```

Exemple : pour atteindre 500 t/min PTO sur un attelage 540/1944 :
`500 × (1944 / 540) = 1800 t/min moteur`.

### 6.2 Vitesse au sol sur un rapport

```
v_sol = v_max_rapport × (rpm_moteur / rpm_max)
```

La table de vitesses étant définie au régime max, on obtient la vitesse au régime moteur courant par simple ratio.

### 6.3 Débit requis

Le débit total à pulvériser pour respecter la dose à l'hectare est :

```
débit_L/min = (L_ha × v_km/h × largeur_m) / 600
```

Le facteur 600 vient de :
`(L/ha × km/h × m) / (10 000 m²/ha × 60 min/h) = (L × km × m) / (60 × 10 000 × h × m²) → /600`.

Le débit **par buse** est ce débit total divisé par le nombre de buses.

### 6.4 Pression

La pression est obtenue par **interpolation linéaire** dans la table d'étalonnage entre les deux points encadrants. Si le débit par buse sort de la plage, la pression n'est pas calculée et une alerte est levée.

### 6.5 Plage PTO et fenêtre exploitable

Pour chaque rapport, le calcul est réalisé aux **trois régimes PTO** : min, nominal, max. On obtient ainsi une **fenêtre vitesse / pression** dans laquelle le réglage est valide — au lieu d'une valeur unique. C'est cette fenêtre qui détermine si un rapport est utilisable pour la dose visée.

---

## 7. Bonnes pratiques

- **Vérifier la table de vitesses** : elle dépend des pneus et de la transmission. Mesurer avec un GPS sur 100 m de plat est une bonne sanity-check.
- **Étalonner les buses au moins une fois par saison** : usure céramique, dépôts, dérive de la pompe modifient la courbe. Un récipient gradué et un chrono suffisent.
- **Choisir un rapport avec marge** : préférer un rapport dont la fenêtre PTO est centrée dans la plage d'étalonnage, plutôt qu'aux bornes.
- **Doses** : le catalogue produits sert de mémoire d'atelier. Toujours croiser avec la fiche technique du produit avant chaque traitement.

---

## 8. Dépannage

| Symptôme | Vérifier |
|---|---|
| Tous les rapports en alerte *P. haute* | Largeur de travail ou nombre de buses incohérent ; dose L/ha trop élevée pour la table d'étalonnage |
| *P. basse* sur les petits rapports | Normal pour des doses faibles : monter d'un rapport |
| Débit par buse incohérent | Vérifier nombre de buses dans la configuration |
| Pression non affichée (`-`) | Débit hors plage d'étalonnage : compléter la table avec un point plus bas / plus haut |
| Aucun produit dans la liste | Catalogue vide : ajouter via l'onglet **Produits** |

---

## 9. Pour aller plus loin

- Le module `src/calc.rs` est isolé du reste de l'application et entièrement testé. Pour l'utiliser comme bibliothèque dans un autre outil (CLI, script), il suffit d'extraire les structures `Config` / `ResultatCalcul` et la fonction `calculer`.
- Le format TOML de configuration est versionnable : on peut maintenir plusieurs jeux de configuration (un par tracteur) en remplaçant le fichier dans `~/.config/pulve-calc/`.
