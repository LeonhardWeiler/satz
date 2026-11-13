# UI-Plan: Audit 2 (2026-11)

Grundlage: Shot-Suite `web/shots` (90 Tests, Light und Dark, 2× DPR) gegen `web/dist`.
Erledigte Punkte werden gelöscht; was umgesetzt wurde, steht am Ende.

## Leitidee

Ein Baustein je Aufgabe, ein Wert je Entscheidung. Jede Steuerung kommt aus
`controls.tsx`, jede Größe aus den Skalen in `design.test.ts`, jede Farbe aus einem
Token in `:root`. Gleiche Aufgabe sieht überall gleich aus (Segmented ist Segmented,
Feld ist Feld), Zustände (hover, pressed, selected, disabled, focus) haben je genau
eine Regel. Light und Dark teilen alle Regeln und unterscheiden sich nur in Tokens.

## Offen

Nach 5 Runden keine offenen Befunde. Prüfung je Runde: volle Shot-Suite (90 Tests) in
Light und Dark, dazu 900 px Fensterbreite.

## Bewusst nicht

- Opacity behält ihr Icon-Label wie die anderen Metrik-Felder.
- Farbmodus bleibt Select im Dokument-Panel (Umrechnung ist gewichtig).
- Find-Buttons behalten den Rand wie jeder `.button`.
- Ein deaktiviertes Select dimmt nur den Pfeil, der Wert bleibt lesbar.
- `.kv strong` bleibt fett (Schlüssel-Anzeige), `.iss`-Auswahl bleibt Innenring.
- Paint-Zeilen lassen rechts Platz für die Hover-Aktionen.
- Textstile bleiben aufklappbare Zeilen mit fettem Namen (sie tragen Einstellungen),
  Variablen und Swatches öffnen einen Editor.
