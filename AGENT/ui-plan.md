# UI-Plan: Audit 3 (2026-11)

Grundlage: Shot-Suite `web/shots` (292 Shots je Modus, Light und Dark, 2× DPR) und
`audit.spec.ts`. Leitidee: ein Baustein je Aufgabe, ein Token je Farbe, eine Regel je
Zustand; Light und Dark unterscheiden sich nur in Tokens.

## Offen (Reihenfolge)

1. Tokens
   - Dark: schwarze Seiten verschwinden auf `--canvas`. Haarlinie um die Trim-Box jeder
     Seite im Renderer, Farbe aus `--page-edge`; dasselbe Token umrandet `.ov-page`.
   - Dark: schwarze Chips in Swatch-, Variablen- und Picker-Listen ohne Innenkante;
     eine Chip-Regel mit der Innenkante des Fill-Swatch.
2. Fokus: die Ebenen-Zeile zeigt den Ring um die ganze Zeile (Radius 6), nicht eckig um
   den Namen. [beide]
3. Suchtreffer: Palette (`<b>`) und Shortcuts (`<mark>`) teilen eine Regel, Treffer 600,
   Rest `--text`.
4. Find: der aktuelle Treffer hebt sich ab (andere Treffer schwächer); „Style any“ → „Any“.
5. Neues-Dokument-Dialog: Höhe folgt dem Inhalt, „Preset Preset 1“ nicht doppelt,
   Icon-Buttons im Kopf auf der Inhaltskante.
6. Properties: „Align left“ auf der rechten Kante wie der Fill-Swatch (4 px Versatz).
7. `AGENT/design.md`: veraltete Regel zur Einheiten-Endung streichen.

Punkte sind unabhängig; 1 zuerst, weil es Canvas- und Overview-Shots ändert. Nach jedem
Punkt die Shots der Kategorie, nach jeder Runde alle.

## Bewusst nicht

- Opacity behält ihr Icon-Label wie die anderen Metrik-Felder.
- Farbmodus bleibt Select im Dokument-Panel (Umrechnung ist gewichtig).
- Find-Buttons behalten den Rand wie jeder `.button`.
- Ein deaktiviertes Select dimmt nur den Pfeil, der Wert bleibt lesbar.
- `.kv strong` bleibt fett (Schlüssel-Anzeige), `.iss`-Auswahl bleibt Innenring.
- Paint-Zeilen lassen rechts Platz für die Hover-Aktionen.
- Textstile bleiben aufklappbare Zeilen mit fettem Namen (sie tragen Einstellungen),
  Variablen und Swatches öffnen einen Editor.
- Das W/H-Schloss sitzt in der 8-px-Lücke (Brücke).
