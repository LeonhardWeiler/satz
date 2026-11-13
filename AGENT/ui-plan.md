# UI-Plan: Audit 3 (2026-11)

Grundlage: Shot-Suite `web/shots` (293 Shots je Modus, Light und Dark, 2× DPR) und
`audit.spec.ts`. Leitidee: ein Baustein je Aufgabe, ein Token je Farbe, eine Regel je
Zustand; Light und Dark unterscheiden sich nur in Tokens.

## Offen

Runde 2 (Pixel-Diff s0→s1, Audit): nichts offen.

## Umgesetzt

- Canvas: jede Seite zeigt ihre Trim-Kante in der Farbe der Schnittmarken; schwarze
  Seiten verschwinden im Dark Mode nicht mehr. Übersicht: Seiten mit `--edge` umrandet.
- Ebenen: der Tastaturfokus umrandet die ganze Zeile (Radius 6); Shot „05b keyboard focus“.
- Suchtreffer: Palette und Shortcuts teilen eine Regel (Treffer 600, Rest `--text`).
- Find: andere Treffer schwächer (0.4), der aktuelle hebt sich ab; Stilfilter „Any“.
- Neues Dokument: Höhe folgt dem Inhalt (max. 640 px), Feld „Name“ statt „Preset“.
- `design.md`: veraltete Regel zur Einheiten-Endung gestrichen.

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
- Benachbart ausgewählte Ebenen bleiben einzelne Pillen.
- Audit-Kantenversatz bei verschieden breiten Nachbarn (24 vs. 28 px) ist kein Fehler;
  die bündige Kante zählt.
- Chips in Listen haben ihre Innenkante, auch schwarze sind im Dark Mode sichtbar.
- Shot „28 exporting“ zeigt einen Rest-Kasten; er stammt vom Zuschnitt in `shot()`, der
  volle Screenshot ist sauber.
