
## Mål
### Spelmotor
- Basic 3D Pipeline
- Physics (Hard body)
- Scriptingspråk
- Stöd för compute shaders eller liknande
- Stöd för ljud (.wav)
- dunderbra prestanda
- Hyfsat optimerad och trevlig ECS
### IDE
- Editor för spelmotor med integrations för code editors(vs code etc)
- Inställningar för theme och keybinds
- skitsnygg logga
- ta över världen

## MVP (Minimum viable product)
### Spelmotor
- Simpel och ooptimerad ECS
- Simpel 3D-grafik
- Möjlighet att ladda in modeller
### IDE
- Runtime controls
- World Viewport med interaction(raycasted cursor)
- Inspect entities (och klasser, basically info dumpa allt, ska va clickable för att komma till rätt kod-del för objektet)
- Default edit controls(copy, paste, cut, undo, redo etc)
- Flytta/Skala/Rotera objekt (bild)
- Spara hela projektet och öppna från filer/fil

## Roller (Vem som gör vad)
### Spelmotor
- Vincent (Fysik)
- Mathias (ECS)
- Theodor
## IDE
- Lukas
- Daniel
Projektledning?
Olle J (Kan tänka sig)
Mathias (Kan offra sig om Olle dör, men Olle är Vaccinerad)
Workflow/CI CD?
Olle J
Lukas
Scriptingspråk
Mathias

Tekniker
Spelmotor
Rust
Vulkan, Ash (rätta gärna)
IDE
https://github.com/emilk/egui


