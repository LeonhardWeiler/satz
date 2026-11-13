# UI-Plan: Audit 2 (2026-11)

Grundlage: Shot-Suite `web/shots` (90 Tests, Light und Dark, 2× DPR, dazu 900 px Breite)
gegen `web/dist`, dazu das Mess-Audit `web/shots/audit.spec.ts` (Kanten, Höhen,
Text-Einzüge, Schriften, Abstände). 6 Runden, deployt und geprüft am 2026-11-13.

## Leitidee

Ein Baustein je Aufgabe, ein Wert je Entscheidung. Jede Steuerung kommt aus
`controls.tsx`, jede Größe aus den Skalen in `design.test.ts`, jede Farbe aus einem
Token in `:root`. Gleiche Aufgabe sieht überall gleich aus, Zustände (hover, pressed,
selected, disabled, focus) haben je genau eine Regel. Light und Dark teilen alle Regeln
und unterscheiden sich nur in Tokens.

## Umgesetzt

- Shortcuts: Bold, Italic, Underline, Strikethrough, Ausrichtung und Schriftgröße per
  Taste auf Auswahl oder markiertem Text; `a` als zweite Frame-Taste und Ctrl R (Rename
  doppelt zu F2) entfernt; Ctrl Shift , öffnet nicht mehr die Settings.
- Keep aspect ratio wieder zwischen W und H, verbunden zeigt eine Brücke beide Felder als
  ein Paar.
- Tabellen: Zellraster auf dem Canvas, solange Tabelle oder Zelle gewählt ist; Zeilen und
  Spalten über + und ⋯ im Kopf, dieselben Aktionen im Kontextmenü (Table) und in der
  Palette; Felder „Column“/„Row“; Radius, Clip und Auto layout nicht bei Tabellen;
  Zellen zeigen „n cells“, keine Layout-Sektion und nur Ausrichtung und Inset; leere
  Zellen heißen A1, B1 …
- `MenuButton` in `controls.tsx` für Kopf-Menüs, auch Fit image nutzt ihn.
- GREP-Zeile auf dem 2-Spalten-Raster, ohne Label; Inhaltsverzeichnis mit Überschrift
  „Listed text styles“; Verankerung als „Inline | Custom“ neben „Release anchor“.

- Messrunde (6): jede Box im Panel sitzt auf der 12-px-Inhaltskante, links und rechts
  (Tabs, Baum, Zeilen, Icon-Buttons im Kopf); Felder, Selects, Buttons und Farbchips
  28 px, Icon-Buttons 24 px mittig in 28-px-Zeilen; Abstände 8 zwischen Steuerungen,
  4 zwischen Icon-Buttons, 8 in Popovers; Preflight- und Properties-Kopf gleich hoch;
  Zeilenaktionen immer sichtbar; Ebenen-Lock/Auge, Stil- und Zeilen-Löschen sind
  `.icon-button`; Master-Vorschauen so breit wie „New master“; schmales Fenster:
  linkes Panel 200 px, Inhalt passt mit Einzug.

- Zustände: `--dim` ist der einzige Disabled-Wert, eine Regel dimmt jede deaktivierte
  Steuerung; Tabs und Segmented teilen Fläche, Auswahl und Hover.
- Panel: Layout auf einem 2-Spalten-Raster; Felder ohne
  `disabled` (Drop-Cap-Zeichen erst ab 1 Zeile); Einzüge in der Reihenfolge Left, Right,
  Before, After, First line; kurze Labels („Inner“), voller Name im Tooltip.
- Felder: Namensfelder (Swatch, Variable, Modus, Tab-Leader, GREP, Präfix) sitzen in
  `.field` mit 4 px Abstand zum Label wie jedes `Field`.
- Dialoge: Settings fokussiert Suche bzw. aktuelle Seite; Bleed steht bei den Rändern im
  Neues-Dokument-Dialog; Titel der Versionshistorie mittig im Kopf.
- Menüs und Listen: Auswahl per Häkchen statt fett; Swatch-Löschen ist `.icon-button`.
- Pflege: doppelte und tote CSS-Regeln entfernt (`index.css` −127 Zeilen netto);
  `AGENT/design.md` beschreibt das CSS; Shots für schmales Fenster, Versionshistorie und
  Textstile im Assets-Tab.

## Bewusst nicht

- Opacity behält ihr Icon-Label wie die anderen Metrik-Felder.
- Farbmodus bleibt Select im Dokument-Panel (Umrechnung ist gewichtig).
- Find-Buttons behalten den Rand wie jeder `.button`.
- Ein deaktiviertes Select dimmt nur den Pfeil, der Wert bleibt lesbar.
- `.kv strong` bleibt fett (Schlüssel-Anzeige), `.iss`-Auswahl bleibt Innenring.
- Paint-Zeilen lassen rechts Platz für die Hover-Aktionen.
- Textstile bleiben aufklappbare Zeilen mit fettem Namen (sie tragen Einstellungen),
  Variablen und Swatches öffnen einen Editor.
