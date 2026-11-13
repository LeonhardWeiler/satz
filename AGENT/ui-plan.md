# UI-Plan: Audit 2 (2026-11)

Grundlage: Shot-Suite `web/shots` (89 Tests, Light und Dark, 2× DPR) gegen `web/dist`.
Erledigte Punkte werden gelöscht; was umgesetzt wurde, steht am Ende.

## Leitidee

Ein Baustein je Aufgabe, ein Wert je Entscheidung. Jede Steuerung kommt aus
`controls.tsx`, jede Größe aus den Skalen in `design.test.ts`, jede Farbe aus einem
Token in `:root`. Gleiche Aufgabe sieht überall gleich aus (Segmented ist Segmented,
Feld ist Feld), Zustände (hover, pressed, selected, disabled, focus) haben je genau
eine Regel. Light und Dark teilen alle Regeln und unterscheiden sich nur in Tokens.

## Offen

### Panel
1. Feldlabels passen in eine halbe Spalte bei 900 px Fensterbreite (kein abgeschnittener
   Wert); lange Namen gehören in `title`.

### Dialoge
2. Kopfzeilen von Palette, Find und History: Titel und Eingabe auf einer Mittellinie.

### Light / Dark
3. Shots in beiden Themes nach jeder Runde vollständig durchsehen; bisher ohne
   Theme-spezifischen Befund offen.

### Pflege
4. Shot-Suite deckt jeden Dialog ab (History ergänzt, schmales Fenster ergänzt).

## Bewusst nicht

- Opacity behält ihr Icon-Label wie die anderen Metrik-Felder.
- Farbmodus bleibt Select im Dokument-Panel (Umrechnung ist gewichtig).
- Find-Buttons behalten den Rand wie jeder `.button`.
- Ein deaktiviertes Select dimmt nur den Pfeil, der Wert bleibt lesbar.
- `.kv strong` bleibt fett (Schlüssel-Anzeige), `.iss`-Auswahl bleibt Innenring.
- Paint-Zeilen lassen rechts Platz für die Hover-Aktionen.

## Abhängigkeiten und Reihenfolge

4 vor jeder Runde (Shots sind die Prüfgrundlage); 1 und 2 unabhängig; 3 zuletzt je Runde.
