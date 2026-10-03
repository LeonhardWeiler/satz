# Satz im Vergleich

Stand 02.10.2026, Satz bei `e066fd3`. Verglichen mit Adobe InDesign, Affinity
(Publisher/Designer, seit 2025 eine App), QuarkXPress, Scribus, Figma, Penpot,
Canva und Adobe Illustrator. Die Funktionen der anderen Programme stammen aus
meinem Wissen über ihre aktuellen Versionen und sind nicht einzeln in den
Programmen nachgeprüft. „Hat Satz“ wurde im Code geprüft.

## Die Programme kurz

| Programm    | Zielgruppe                | Stärke gegenüber Satz                                                                                                                   |
| ----------- | ------------------------- | --------------------------------------------------------------------------------------------------------------------------------------- |
| InDesign    | Verlage, Agenturen, Druck | Langdokumente (Bücher, Inhaltsverzeichnis, Index, Fußnoten), GREP- und verschachtelte Stile, Tabellen, Preflight-Profile, IDML, Skripte |
| Affinity    | Grafiker, Selbständige    | Raster, Vektor und Layout in einer Datei, Live-Filter, IDML- und PDF-Import, Datenzusammenführung                                       |
| QuarkXPress | Verlage                   | Ähnlich InDesign, plus HTML5-Export, Umwandlung von PDF in native Objekte                                                               |
| Scribus     | Open Source               | PDF/X und Farbseparation, Python-Skripte, Tabellen, Rendering-Frames (LaTeX)                                                            |
| Figma       | UI-Design, Teams          | Echtzeit-Zusammenarbeit, Komponenten und Varianten, Bibliotheken, Plugins, Kommentare, Grid-Auto-Layout                                 |
| Penpot      | Open-Source-Figma         | Flex- und Grid-Layout wie in CSS, Selbst-Hosting, SVG als Dateiformat                                                                   |
| Canva       | Laien, Marketing          | Vorlagen, Markenpaket, „Magic Resize“, Bild-KI, Hintergrund entfernen                                                                   |
| Illustrator | Vektorgrafik              | Pinsel, Formerstellung, variable Strichbreiten, Verlaufsgitter, Vektorisieren, Text auf Pfad                                            |

Satz hat davon schon: Musterseiten mit Überschreiben, verkettete Rahmen,
Knuth-Plass-Umbruch mit Silbentrennung (de, en), Grundlinienraster, Spalten,
Text- und Absatzstile, OpenType-Features, CMYK, Volltonfarben, Farbvariablen
mit Modi, Separationen, Farbauftrag, Gamut-Warnung, PDF/X-4 und X-1a mit
Beschnitt und Schnittmarken, Auto Layout, Constraints, Masken, boolesche
Operationen, Pfadbearbeitung mit Fülleimer, gestrichelte, gepunktete, wellige und Zickzack-Linien,
Text in Konturen, Bildzuschnitt und Bildanpassung, Hilfslinien, Snapping mit
Abständen, Befehlspalette, frei belegbare Tastenkürzel.

## Was fehlt

Wert für Satz: **hoch** = typische Aufgaben im Ziel (Plakat, Broschüre, Druck)
scheitern ohne die Funktion; **mittel** = spart viel Zeit oder fällt Profis auf;
**niedrig** = nett, nischig oder außerhalb des Ziels. „Liste“ zeigt, wo der Punkt
schon steht: T = `TODO.md` Later, B = `bugs.md` LATER.

### Text und Typografie

| Funktion                                                                                            | Wer hat sie                              | Wert    | Liste     |
| --------------------------------------------------------------------------------------------------- | ---------------------------------------- | ------- | --------- |
| Aufzählungen und Nummerierung                                                                       | alle                                     | hoch    | T         |
| Tabulatoren mit Tabstopps (links, rechts, zentriert, Dezimal, Füllzeichen)                          | InDesign, Affinity, Quark, Scribus       | hoch    |           |
| Tabellen                                                                                            | InDesign, Affinity, Quark, Scribus       | hoch    | B         |
| Textumfluss um Objekte                                                                              | InDesign, Affinity, Quark, Scribus       | hoch    | T         |
| Suchen und Ersetzen                                                                                 | alle                                     | hoch    | T         |
| Rechtschreibprüfung                                                                                 | alle außer Penpot                        | hoch    |           |
| Initialen (Drop Caps)                                                                               | InDesign, Affinity, Quark, Scribus       | mittel  |           |
| Absatzumbruch-Optionen: Hurenkinder, Schusterjungen, „nicht trennen“, „mit nächstem zusammenhalten“ | InDesign, Affinity, Quark, Scribus       | hoch    |           |
| Zeichenformate getrennt von Absatzformaten, „nächstes Format“                                       | InDesign, Affinity, Quark, Scribus       | mittel  |           |
| Verschachtelte und GREP-Stile                                                                       | InDesign, Affinity (teilweise)           | niedrig |           |
| Hochgestellt, tiefgestellt, Grundlinienversatz, manuelles Kerning                                   | alle Layoutprogramme                     | mittel  |           |
| Optischer Randausgleich, hängende Interpunktion                                                     | InDesign, Affinity                       | mittel  |           |
| Absatzlinien und -schattierung, Rahmen um Absätze                                                   | InDesign, Affinity                       | niedrig |           |
| Spaltenspannende Absätze, ausgeglichene Spalten                                                     | InDesign, Affinity                       | mittel  |           |
| Text auf Pfad                                                                                       | InDesign, Affinity, Scribus, Illustrator | mittel  |           |
| Verankerte Objekte im Text                                                                          | InDesign, Affinity, Quark                | mittel  | B         |
| Fußnoten und Endnoten                                                                               | InDesign, Affinity, Quark                | mittel  |           |
| Inhaltsverzeichnis aus Absatzformaten                                                               | InDesign, Affinity, Scribus              | mittel  | B         |
| Index, Querverweise, lebende Kolumnentitel                                                          | InDesign, Affinity                       | niedrig |           |
| Textvariablen (Datum, Dateiname, Kapitel)                                                           | InDesign, Affinity                       | mittel  | B (Datum) |
| Bedingter Text                                                                                      | InDesign                                 | niedrig | B         |
| Glyphenpalette, Achsen variabler Schriften                                                          | InDesign, Affinity, Figma                | mittel  | T         |
| Weitere Silbentrennungssprachen (fr, it, es, nl …)                                                  | alle                                     | mittel  |           |
| Text aus Word, RTF, Markdown platzieren                                                             | InDesign, Affinity, Quark, Scribus       | mittel  |           |
| Textmodus-Editor (Story Editor)                                                                     | InDesign                                 | niedrig |           |
| Vertikaler Text, Rechts-nach-links                                                                  | InDesign ME, Affinity (teilweise)        | niedrig |           |

### Formen, Pfade und Bilder

| Funktion                                                        | Wer hat sie                          | Wert    | Liste |
| --------------------------------------------------------------- | ------------------------------------ | ------- | ----- |
| Spiegeln horizontal und vertikal                                | alle                                 | hoch    |       |
| Neigen (Scheren)                                                | InDesign, Affinity, Illustrator      | mittel  |       |
| Freihand-Bleistift und Pinsel                                   | Figma, Affinity, Illustrator, Penpot | mittel  |       |
| Formerstellung durch Ziehen über Flächen                        | Illustrator, Affinity                | niedrig |       |
| Variable Strichbreite, Pinselspitzen                            | Illustrator, Affinity                | niedrig |       |
| Eckenglättung (Squircle)                                        | Figma                                | niedrig |       |
| Kegel- und Rautenverlauf, Verlaufsgitter                        | Figma, Affinity, Illustrator         | niedrig |       |
| Effekte: Schatten innen, Schein, Hintergrundunschärfe, Rauschen | Figma, Affinity, InDesign            | mittel  |       |
| Bild in Rahmen: Füllen, Einpassen, Kacheln, proportional        | Figma, InDesign, Affinity            | hoch    |       |
| SVG und PDF als Vektoren platzieren                             | alle                                 | hoch    | T     |
| Verknüpfte Bilder mit Neu-Verknüpfen                            | InDesign, Affinity, Quark, Scribus   | mittel  | T     |
| Freistellungspfad aus Alphakanal, Beschneidungspfad             | InDesign, Affinity                   | mittel  |       |
| PSD, TIFF platzieren                                            | InDesign, Affinity, Quark, Scribus   | mittel  |       |
| Hintergrund entfernen, Bild vektorisieren                       | Canva, Illustrator, Affinity         | niedrig |       |
| Pipette                                                         | alle                                 | hoch    |       |
| Schritt und Wiederholen, Raster aus Kopien                      | InDesign, Affinity, Figma (Plugins)  | mittel  |       |
| Transformation wiederholen                                      | InDesign, Illustrator, Affinity      | niedrig |       |

### Dokument und Layout

| Funktion                                                                  | Wer hat sie                          | Wert    | Liste      |
| ------------------------------------------------------------------------- | ------------------------------------ | ------- | ---------- |
| Abschnitte mit eigener Seitennummerierung (römisch, Präfix, Startzahl)    | InDesign, Affinity, Quark, Scribus   | hoch    |            |
| Seiten und Doppelseiten verschieben                                       | alle                                 | mittel  | B          |
| Objektformate (gespeicherte Füllung, Kontur, Effekte, Textrahmenoptionen) | InDesign, Figma (Styles), Affinity   | mittel  |            |
| Komponenten und Instanzen, Bibliotheken                                   | Figma, Penpot, Affinity (Symbole)    | mittel  |            |
| Auto Layout mit Umbruch (Wrap) und Grid-Layout                            | Figma, Penpot                        | mittel  |            |
| Ebenen über das ganze Dokument (wie InDesign-Ebenen)                      | InDesign, Affinity, Scribus          | niedrig |            |
| Formatwechsel mit Anpassung (Magic Resize, Liquid Layout)                 | Canva, InDesign                      | mittel  |            |
| Datenzusammenführung (Serienbrief aus CSV)                                | InDesign, Affinity, Scribus (Skript) | mittel  |            |
| Buchdateien aus mehreren Dokumenten                                       | InDesign, Affinity                   | niedrig |            |
| Mehrere offene Dokumente, Projektübersicht                                | alle                                 | mittel  | B          |
| Skripte und Plugins                                                       | InDesign, Scribus, Figma, Penpot     | niedrig |            |
| Kommentare, Echtzeit-Zusammenarbeit                                       | Figma, Penpot, Canva                 | mittel  | B (Server) |
| Versionsverlauf                                                           | Figma, Penpot, Canva                 | mittel  | T          |

### Farbe und Druck

| Funktion                                                                                                    | Wer hat sie                        | Wert    | Liste         |
| ----------------------------------------------------------------------------------------------------------- | ---------------------------------- | ------- | ------------- |
| Überdrucken (Fläche, Kontur) mit Überdruckenvorschau                                                        | InDesign, Affinity, Quark, Scribus | hoch    |               |
| Eigene CMYK-Profile                                                                                         | alle Layoutprogramme               | mittel  | T             |
| Druckermarken über Schnittmarken hinaus: Passermarken, Farbkontrollstreifen, Seiteninfo, Infobereich (Slug) | InDesign, Affinity, Quark, Scribus | mittel  |               |
| PDF-Optionen: Seitenbereich, Bildneuberechnung und Kompression, Druckbögen statt Seiten                     | InDesign, Affinity, Quark, Scribus | hoch    |               |
| Interaktives PDF: Links, Lesezeichen aus Überschriften                                                      | InDesign, Affinity, Quark, Scribus | mittel  |               |
| Barrierefreies PDF (getaggt, Alternativtexte)                                                               | InDesign, Affinity                 | mittel  |               |
| PNG-, JPEG-, SVG-Export                                                                                     | alle                               | hoch    | T (PNG, JPEG) |
| EPUB- und HTML-Export                                                                                       | InDesign, Quark                    | niedrig |               |
| Verpacken für die Druckerei                                                                                 | InDesign, Affinity, Quark, Scribus | mittel  | T             |
| Ausschießen (Broschürendruck)                                                                               | InDesign, Affinity                 | mittel  |               |
| Preflight-Profile, Live-Preflight mit Regeln                                                                | InDesign                           | niedrig |               |
| Farbmischung aus Volltonfarben (Mixed Ink)                                                                  | InDesign                           | niedrig |               |

### Import und Austausch

| Funktion                  | Wer hat sie                          | Wert    | Liste |
| ------------------------- | ------------------------------------ | ------- | ----- |
| IDML-Import               | Affinity, Quark, Scribus (teilweise) | mittel  |       |
| PDF öffnen und bearbeiten | Affinity, Quark, Scribus             | niedrig |       |
| Figma-Import              | Penpot                               | mittel  | B     |
| Offline als App           | Desktop-Programme, Figma (Desktop)   | mittel  | T     |

## Empfehlung

Die größten Lücken für Plakat und Broschüre, nach Wert und Aufwand. Was schon
in `TODO.md` steht, ist nicht wiederholt.

1. **Absatzumbruch-Optionen** (Hurenkinder, Schusterjungen, zusammenhalten): Der
   Knuth-Plass-Umbruch und der Flow über Rahmen sind da, es fehlen Regeln im Flow.
2. **Tabulatoren**: Preislisten, Inhaltsverzeichnisse und Formulare brauchen sie
   vor Tabellen.
3. **Spiegeln und Pipette**: klein, und jeder Nutzer sucht sie.
4. **Bild in Rahmen einpassen und füllen**: der Zuschnitt ist da, es fehlen die
   Befehle, die ihn ausrechnen.
5. **Überdrucken mit Vorschau**: für Druckdaten (schwarzer Text auf Farbflächen)
   üblich, und die Separationen im Preflight zeigen es dann richtig.
6. **Abschnitte mit Seitennummerierung**: Broschüren mit Umschlag beginnen selten
   bei 1.
7. **PDF-Optionen**: Seitenbereich und Bildneuberechnung, weil Fotos sonst die PDFs
   aufblähen.
8. **Rechtschreibprüfung**: Der Browser kann sie im Texteditor nicht, weil Satz den
   Text selbst setzt. Eine Wortliste pro Sprache (Hunspell-Daten) in der Engine
   wäre der Weg.

Bewusst nicht empfohlen: GREP-Stile, Skripte, Buchdateien, EPUB, Verlaufsgitter,
Vektorisieren. Sie kosten viel und passen nicht zum Ziel „Plakat und Broschüre“.
