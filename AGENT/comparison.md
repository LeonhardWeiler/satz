# Satz im Vergleich

Stand 05.10.2026, Satz bei `5df7a25`. Verglichen mit Adobe InDesign, Affinity
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
Knuth-Plass-Umbruch mit Silbentrennung (de, en, fr, it, es, nl),
Absatzumbruch-Optionen (Zeilen zusammenhalten, Absatz nicht trennen, mit nächstem
zusammenhalten), Aufzählungen und Nummerierung, Tabulatoren mit Füllzeichen,
Initialen, Einzüge, maximale Zeilenzahl, Versalhöhe als Rahmenoberkante,
Grundlinienraster, Spalten, Text- und Absatzstile, Hoch- und Tiefstellung,
Grundlinienversatz, OpenType-Features, Glyphenpalette, Suchen und Ersetzen,
Textumfluss um Objekte (umfließen, überspringen), lokale Schriften und WOFF/WOFF2,
CMYK, Volltonfarben, eigene CMYK-Profile, Überdrucken, Farbvariablen mit Modi,
Separationen, Farbauftrag, Gamut-Warnung, PDF/X-4 und X-1a mit Beschnitt und
Schnittmarken, PDF-Seitenbereich und JPEG-Neuberechnung, PNG- und JPEG-Export,
Verpacken als ZIP, Abschnitte mit eigener Seitennummerierung, Seiten verschieben,
Auto Layout, Constraints, Masken, boolesche Operationen, Pfadbearbeitung mit
Fülleimer, Spiegeln, Pipette, Bild füllen und einpassen, gestrichelte,
gepunktete, wellige und Zickzack-Linien, Text in Konturen, Bildzuschnitt und
Bildanpassung, Hilfslinien, Snapping mit Abständen, Befehlspalette, frei belegbare
Tastenkürzel, offline als PWA.

## Was fehlt

Wert für Satz: **hoch** = typische Aufgaben im Ziel (Plakat, Broschüre, Druck)
scheitern ohne die Funktion; **mittel** = spart viel Zeit oder fällt Profis auf;
**niedrig** = nett, nischig oder außerhalb des Ziels. „Liste“ zeigt, wo der Punkt
schon steht: B = `bugs.md` LATER.

### Text und Typografie

| Funktion                                                      | Wer hat sie                              | Wert    | Liste     |
| ------------------------------------------------------------- | ---------------------------------------- | ------- | --------- |
| Tabellen                                                      | InDesign, Affinity, Quark, Scribus       | hoch    | B         |
| Rechtschreibprüfung                                           | alle außer Penpot                        | hoch    |           |
| Zeichenformate getrennt von Absatzformaten, „nächstes Format“ | InDesign, Affinity, Quark, Scribus       | mittel  |           |
| Manuelles Kerning                                             | alle Layoutprogramme                     | mittel  |           |
| Optischer Randausgleich, hängende Interpunktion               | InDesign, Affinity                       | mittel  |           |
| Spaltenspannende Absätze, ausgeglichene Spalten               | InDesign, Affinity                       | mittel  |           |
| Text auf Pfad                                                 | InDesign, Affinity, Scribus, Illustrator | mittel  |           |
| Verankerte Objekte im Text                                    | InDesign, Affinity, Quark                | mittel  | B         |
| Fußnoten und Endnoten                                         | InDesign, Affinity, Quark                | mittel  |           |
| Inhaltsverzeichnis aus Absatzformaten                         | InDesign, Affinity, Scribus              | mittel  | B         |
| Textvariablen (Datum, Dateiname, Kapitel)                     | InDesign, Affinity                       | mittel  | B (Datum) |
| Achsen variabler Schriften                                    | InDesign, Affinity, Figma                | mittel  | B         |
| Text aus Word, RTF, Markdown platzieren                       | InDesign, Affinity, Quark, Scribus       | mittel  |           |
| Verschachtelte und GREP-Stile                                 | InDesign, Affinity (teilweise)           | niedrig | B         |
| Absatzlinien und -schattierung, Rahmen um Absätze             | InDesign, Affinity                       | niedrig |           |
| Index, Querverweise, lebende Kolumnentitel                    | InDesign, Affinity                       | niedrig |           |
| Bedingter Text                                                | InDesign                                 | niedrig | B         |
| Textmodus-Editor (Story Editor)                               | InDesign                                 | niedrig |           |
| Vertikaler Text, Rechts-nach-links                            | InDesign ME, Affinity (teilweise)        | niedrig |           |

### Formen, Pfade und Bilder

| Funktion                                                        | Wer hat sie                          | Wert    | Liste |
| --------------------------------------------------------------- | ------------------------------------ | ------- | ----- |
| SVG und PDF als Vektoren platzieren                             | alle                                 | hoch    | B     |
| Neigen (Scheren)                                                | InDesign, Affinity, Illustrator      | mittel  |       |
| Freihand-Bleistift und Pinsel                                   | Figma, Affinity, Illustrator, Penpot | mittel  |       |
| Effekte: Schatten innen, Schein, Hintergrundunschärfe, Rauschen | Figma, Affinity, InDesign            | mittel  |       |
| Verknüpfte Bilder mit Neu-Verknüpfen                            | InDesign, Affinity, Quark, Scribus   | mittel  | B     |
| Freistellungspfad aus Alphakanal, Beschneidungspfad             | InDesign, Affinity                   | mittel  |       |
| PSD, TIFF platzieren                                            | InDesign, Affinity, Quark, Scribus   | mittel  |       |
| Schritt und Wiederholen, Raster aus Kopien                      | InDesign, Affinity, Figma (Plugins)  | mittel  |       |
| Bild in Rahmen kacheln                                          | Figma, InDesign, Affinity            | niedrig |       |
| Formerstellung durch Ziehen über Flächen                        | Illustrator, Affinity                | niedrig |       |
| Variable Strichbreite, Pinselspitzen                            | Illustrator, Affinity                | niedrig |       |
| Eckenglättung (Squircle)                                        | Figma                                | niedrig |       |
| Kegel- und Rautenverlauf, Verlaufsgitter                        | Figma, Affinity, Illustrator         | niedrig |       |
| Hintergrund entfernen, Bild vektorisieren                       | Canva, Illustrator, Affinity         | niedrig |       |
| Transformation wiederholen                                      | InDesign, Illustrator, Affinity      | niedrig |       |

### Dokument und Layout

| Funktion                                                                  | Wer hat sie                          | Wert    | Liste      |
| ------------------------------------------------------------------------- | ------------------------------------ | ------- | ---------- |
| Objektformate (gespeicherte Füllung, Kontur, Effekte, Textrahmenoptionen) | InDesign, Figma (Styles), Affinity   | mittel  |            |
| Komponenten und Instanzen, Bibliotheken                                   | Figma, Penpot, Affinity (Symbole)    | mittel  |            |
| Auto Layout mit Umbruch (Wrap) und Grid-Layout                            | Figma, Penpot                        | mittel  |            |
| Formatwechsel mit Anpassung (Magic Resize, Liquid Layout)                 | Canva, InDesign                      | mittel  |            |
| Datenzusammenführung (Serienbrief aus CSV)                                | InDesign, Affinity, Scribus (Skript) | mittel  |            |
| Mehrere offene Dokumente, Projektübersicht                                | alle                                 | mittel  | B          |
| Kommentare, Echtzeit-Zusammenarbeit                                       | Figma, Penpot, Canva                 | mittel  | B (Server) |
| Versionsverlauf                                                           | Figma, Penpot, Canva                 | mittel  | B          |
| Ebenen über das ganze Dokument (wie InDesign-Ebenen)                      | InDesign, Affinity, Scribus          | niedrig |            |
| Buchdateien aus mehreren Dokumenten                                       | InDesign, Affinity                   | niedrig |            |
| Skripte und Plugins                                                       | InDesign, Scribus, Figma, Penpot     | niedrig |            |

### Farbe und Druck

| Funktion                                                                                | Wer hat sie                        | Wert    | Liste |
| --------------------------------------------------------------------------------------- | ---------------------------------- | ------- | ----- |
| SVG-Export                                                                              | alle                               | mittel  |       |
| Druckermarken über Schnittmarken hinaus: Passermarken, Farbkontrollstreifen, Seiteninfo | InDesign, Affinity, Quark, Scribus | mittel  |       |
| Druckbögen statt Seiten im PDF                                                          | InDesign, Affinity, Quark, Scribus | mittel  |       |
| Interaktives PDF: Links, Lesezeichen aus Überschriften                                  | InDesign, Affinity, Quark, Scribus | mittel  |       |
| Barrierefreies PDF (getaggt, Alternativtexte)                                           | InDesign, Affinity                 | mittel  |       |
| Ausschießen (Broschürendruck)                                                           | InDesign, Affinity                 | mittel  |       |
| EPUB- und HTML-Export                                                                   | InDesign, Quark                    | niedrig |       |
| Preflight-Profile, Live-Preflight mit Regeln                                            | InDesign                           | niedrig |       |
| Farbmischung aus Volltonfarben (Mixed Ink)                                              | InDesign                           | niedrig |       |

### Import und Austausch

| Funktion                  | Wer hat sie                          | Wert    | Liste |
| ------------------------- | ------------------------------------ | ------- | ----- |
| IDML-Import               | Affinity, Quark, Scribus (teilweise) | mittel  |       |
| Figma-Import              | Penpot                               | mittel  | B     |
| PDF öffnen und bearbeiten | Affinity, Quark, Scribus             | niedrig |       |

## Empfehlung

Die größten Lücken für Plakat und Broschüre, nach Wert und Aufwand. Die
Empfehlungen 1–7 vom 02.10. (Absatzumbruch-Optionen, Tabulatoren, Spiegeln und
Pipette, Bild einpassen, Überdrucken, Abschnitte, PDF-Optionen) sind umgesetzt.

1. **Rechtschreibprüfung**: Der Browser kann sie im Texteditor nicht, weil Satz den
   Text selbst setzt. Eine Wortliste pro Sprache (Hunspell-Daten) in der Engine
   wäre der Weg; die Silbentrennung kennt die Sprachen schon.
2. **SVG platzieren**: Logos kommen fast immer als SVG; heute müssen sie als PNG
   herein und verlieren im PDF ihre Schärfe.
3. **Tabellen**: Preislisten und Fahrpläne gehen mit Tabulatoren, alles mit
   Zeilenumbruch in Zellen nicht.
4. **Zeichenformate**: Hervorhebungen werden heute pro Textstelle gesetzt und
   lassen sich nicht gemeinsam ändern.
5. **Manuelles Kerning**: Plakattitel brauchen es zwischen einzelnen Buchstaben.
6. **Schatten innen und Neigen**: klein, und Nutzer aus Figma oder Illustrator
   erwarten sie.

Bewusst nicht empfohlen: GREP-Stile, Skripte, Buchdateien, EPUB, Verlaufsgitter,
Vektorisieren. Sie kosten viel und passen nicht zum Ziel „Plakat und Broschüre“.
