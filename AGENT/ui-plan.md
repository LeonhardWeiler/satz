# UI-Plan: eine kohärente Gesamtverbesserung

Grundlage: Screenshots jeder Oberfläche in Light und Dark (1448×1160, Beispieldokument).
Erledigte Punkte sind gelöscht; was umgesetzt wurde, steht am Ende in der Zusammenfassung.

Leitidee: wenige Tokens und Bausteine, jede Oberfläche nur aus diesen gebaut.
`web/src/design.test.ts` erzwingt die Skalen (Schrift, Gewicht, Abstand, Radius,
Schatten, Dauer, rohe Farben nur in `:root`).

## Offen

- Exportfehler-Toast: nur über einen echten Datei- oder Exportfehler auslösbar, Stil
  (`.status.error`) im Code geprüft, nicht als Screenshot.

## Bewusst nicht umgesetzt

| Punkt | Grund |
|---|---|
| Stepper „− n +“ durch Scrub-Felder ersetzen | Ganze Zahlen (Seiten, Spalten) brauchen den Klick-Schritt; Scrub geht zusätzlich. |
| Halbe Zeilen (Rotation, Inner radius, Wrap) | Feste Spalten halten Felder über alle Auswahlen an derselben Stelle. |
| Zweite Ausrichtungszeile „Inside the group“ | Andere Funktion (Kinder im Rahmen), kein Duplikat. |
| Seitenpfeile bei 1 Seite ausblenden | Die Leiste springt sonst beim Anlegen der 2. Seite. |
| Leerzustand für Swatches und Assets | Kopf mit „+“ reicht. |
| Transform als seitliches Untermenü | Das Untermenü ersetzt das Menü mit „›“ und geht mit der Tastatur. |
| Icons in der Palette, Toast unten mittig, Suchleiste bündig | Kein Fehler, nur Geschmack. |
| Stroke-Seiten in die Optionen | Erst vor Kurzem bewusst als Icon in den Kopf gelegt. |
| × im Kopf jedes Popovers | Esc und Klick daneben schließen; der Titel reicht als Kopf. |
| Settings: × im Dialogkopf, kleinere Leerfläche | Das × sitzt schon oben rechts; die feste Höhe verhindert Springen zwischen den Seiten. |
| Font-Menü so breit wie der Auslöser | Ist schon etwa so breit; Namen brauchen Platz. |
| Home: andere Fläche in Light | Home nutzt wie der Editor Canvas- und Panelfarbe, in beiden Themes. |
| Tooltip „Double-click to rename“ über den Tabs | Normaler Tooltip unter dem Auslöser, nur bei Hover. |

## Geprüft und nicht geprüft

Runden 1–5 geprüft (je Light und Dark): Home, Neues Dokument mit Format, leeres
Dokument, Tooltip, Shape-Menü, Eigenschaften aller Beispielebenen, Text mit Type- und
Frame-Optionen, Font-Select, Text-Style-Menü, Insert character, All characters,
Textbearbeitung, Pfadbearbeitung, Umbenennen, Mehrfachauswahl mit Distribute,
Farbwähler Solid und Linear, Stroke-, Layer- und Effekt-Optionen, Auto layout,
Kontextmenüs mit Transform, Assets mit Variablen und Text styles, Swatch anlegen und
bearbeiten, Übersicht mit Kontextmenü, Master bearbeiten, Preflight mit Separationen,
Format und Advanced, Einstellungen, Palette, Suchen, Ansicht ohne UI, Bildfüllung mit
Adjust image, Layout grids mit Menü, Tastaturfokus, Toast mit Undo, Drag-Zustände
(im Code), schmales Fenster (900×800).

## Zusammenfassung: umgesetzt

In 5 Runden, je Light und Dark geprüft, in 6 Commits umgesetzt:

- **Dialoge:** ein Scrim für alle Dialoge; jeder Popover-Dialog hat einen Titel im
  Kopf, Tabs darunter; Optionen und Popover schließen bei neuer Auswahl; Kopf des
  Start-Dialogs wieder gestylt.
- **Panel:** Ebenenname im Panel-Kopf; Datumsfeld mit Label; kein Einheit-Suffix
  nach „Mixed“; Select-Werte enden mit „…“; Swatch-Slot auch für Blur- und Bildzeilen,
  sichtbare Swatch-Ränder.
- **Leisten und Menüs:** gedrückter Übersicht-Button; nur ein Export-Button bei
  offenem Preflight; Häkchen-Toggles im Canvas-Menü; Pfadbearbeitung endet mit einem
  normalen Done-Button wie Zuschneiden und Master.
- **Felder:** Suchfelder (Hilfe, Menüs) sehen aus wie alle anderen Felder, Fokus mit
  Akzentrahmen; flache Format-Kacheln; Shape-Icons nur als Kontur.
- **Canvas:** Beschnittlinien und Suchtreffer in beiden Themes sichtbar.
- **Ziehen:** gezogene Ebenen werden abgeblendet wie gezogene Seiten; Drop-Linien
  überall 2 px.
