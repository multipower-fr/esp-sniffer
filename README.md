
# Documentation carte NodeMCU pour `esp-sniffer`

## Prérequis pour le firmware

- Une carte basée sur l'`ESP8266`
- [Java JDK](https://www.microsoft.com/openjdk)
- L'IDE [ESPlorer](https://github.com/4refr0nt/ESPlorer)
- [`NodeMCU-PyFlasher`](https://github.com/marcelstoer/nodemcu-pyflasher)
- [git](https://git-scm.com/download/win)
- [gh](https://github.com/cli/cli)

## Génération du firmware

Rendez-vous sur le [site officiel](https://nodemcu-build.com/) de création des firmwares NodeMCU

Une fois un email valide entré, choisir les options suivantes dans les choix :

- `file`
- `gpio`
- `net`
- `node`
- `spi`
- `tmr`
- `uart`
- `wifi`
- `wifi_monitor`

Appuyez sur `Start your build`

Vous recevrez un email du site avec deux liens. 

Le firmware à télécharger est le `float`

## Préparation de la carte

Ouvrez le gestionnaire de périphérique (accessible via la recherche Windows), dépliez la section `Ports (COM et LPT)`, et branchez votre carte.  
Une fois la carte branchée, utilisez le menu `Actions` puis `Rechercher des modifications sur le matériel`, une entrée devrait apparaître, gardez le numéro à côté de `COM` en tête

Faites clic-droit sur le port - `Propriétés` - `Paramètres du Port` et vérifiez que les champs sont comme dans ce tableau

| Champ              | Valeur |
| ------------------ | ------ |
| `Bits par seconde` | 115200 |
| `Bits de données`  | 8      |
| `Parité`           | Aucune |
| `Bits d'arrêt`     | 1      |
| `Contrôle de flux` | Aucun  |

## Flash du firmware

Ouvrez `NodeMCU-PyFlasher`

| Champ | Valeur |
| ----- | ------ |
| `Serial Port` | Le port relevé plus tôt (`COMn`) |
| `NodeMCU Firmware` | Le firmware téléchargé |
| `Baud Rate` | 115200 |
| `Erase Flash` | no |

### Passage en mode bootloader 

Sur la carte se trouve des boutons `Boot` et `EN`

Pour passer en mode bootloader, avec la carte branchée au PC, gardez enfoncé le bouton `Boot`, appuyez sur `EN` et relâchez le tout

Vous êtes en mode bootloader et vous pouvez cliquer sur `Flash NodeMCU`

Patientez jusqu'à la fin de la procédure et ensuite, débranchez et rebranchez la carte

## Ajout du script

Ouvrez `ESPlorer.bat` dans le dossier d'ESPlorer

Dans le haut de la fenêtre, choisir le port COM, mettre `115200` dans la vitesse, et vérifiez que `CR` et `LF` est coché

Dans l'en-tête, appuyez sur `Open` et appuyez sur le bouton `Reset`, le terminal devrait afficher des messages

Une fois que ceci est fait, ouvrez une fenêtre de PowerShell (accessible via la recherche Windows)

Si ce n'est pas déjà fait, connectez-vous à l'utilitaire de GitHub `gh`

```ps1
gh auth login
```

Naviguez jusqu'au dossier où vous souhaitez les données et exécutez

```ps1
gh repo clone multipower-fr/esp-sniffer 
```

Retournez dans `ESPlorer`, faites `File`-`Open from Disk` et sélectionnez dans le dossier créé par `gh` le fichier [`sniper.lua`](./sniper.lua)

Une fois celui-ci ouvert, en bas de l'éditeur, utilisez le bouton `Save to ESP`

Faites de même pour [`init.lua`](./init.lua) qui permettra au fichier `sniper.lua` de se lancer au démarrage

Redémarrez votre carte et vous verrez l'arrivée des données dans le terminal. Faites `Close` et vous pourrez maintenant utiliser l'interface PC

## Interface PC

La documentation de l'interface PC est générée par [`rustdoc`](https://doc.rust-lang.org/rustdoc/what-is-rustdoc.html).

Elle peut-être lue dans le code de la crate ([`lib.rs`](./wifisnipe-rs-crate/src/lib.rs)), ou compilée pour consommation par `cargo doc` une fois Rust installé

### Prérequis

- [Visual Studio](https://visualstudio.microsoft.com/) en sélectionnant `Développement Desktop en C++` dans `Charges de Travail` et `Anglais` (en plus de `Français`) dans `Modules Linguistiques`
- [Rustup](https://rustup.rs/), l'installateur de Rust

Ouvrez une fenêtre PowerShell en tant qu'Administrateur et exécuter les commandes suivantes :

```ps1
# Si cela n'a pas été déjà fait : 
# Se connecter a votre compte GitHub
gh auth login
# Cloner la repo
gh repo clone multipower-fr/esp-sniffer

# Installez Rust Stable
rustup toolchain install stable
# Allez dans le dossier de la crate (librairie)
cd esp-sniffer\wifisnipe-rs-crate

# Vous pouvez compiler la documentation dans un format HTML en utilisant
cargo doc --open
```