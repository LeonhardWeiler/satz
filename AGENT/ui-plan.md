# UI-Plan: eine kohärente Gesamtverbesserung

Grundlage: Screenshots jeder Oberfläche in Light und Dark (1448×1160, Beispieldokument).
Erledigte Punkte sind gelöscht; was umgesetzt wurde, steht am Ende in der Zusammenfassung.

Leitidee: wenige Tokens und Bausteine, jede Oberfläche nur aus diesen gebaut.
`web/src/design.test.ts` erzwingt die Skalen (Schrift, Gewicht, Abstand, Radius,
Schatten, Dauer, rohe Farben nur in `:root`).

## Offen

- Runden 2–5: Zustände, die noch nicht als Screenshot geprüft sind (siehe unten).

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

Runde 1 geprüft (je Light und Dark): Home, Neues Dokument mit Format, leeres Dokument,
Tooltip, Shape-Menü, Eigenschaften aller Beispielebenen, Text mit Type- und
Frame-Optionen, Font-Select, Insert character, All characters, Mehrfachauswahl mit
Distribute, Farbwähler Solid und Linear, Stroke-, Layer- und Effekt-Optionen,
Effekt-Typ, Auto layout mit Kind, Kontextmenüs mit Transform, Assets, Übersicht mit
Kontextmenü, Preflight mit Format und Advanced, Einstellungen (alle 3 Seiten,
Theme-Select), Palette mit Suche, Suchen mit Treffer, Color-mode-Select, Ansicht ohne
UI, Swatch anlegen.

Noch nicht geprüft: Text-Style-Menü, Textbearbeitung auf dem Canvas, Pfadbearbeitung,
Status-Toast, Swatch bearbeiten, Variablen und Text styles in Assets, Layout grids,
Master-Seite bearbeiten, Bildfüllung mit Adjust image, Umbenennen im Baum,
Drag-Zustände, Exportfehler, schmale Fenster, Tastaturfokus.
