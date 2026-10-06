--- NOW
- [x] Performance of `settle` and `lay_out`, which walk the whole tree after every command; at 32 pages the snapshot build takes most of the time.
- [x] Kontur innen/außen pro Seite, Absatzlinien oder ‑hintergrund, Textrahmen-Spaltenlinie, Mindesthöhe bei Auto height.
- [x] the bucket tool should only not fill over line crossings. an area is when it is surrounded by lines in all directions, there does not have to be vertices that connect the lines around it. it should work like in indesign. it does not work correctly only sometimes does it work like its supposed to, most of the time it says that there's no area, even though there is
- [x] wenn ich im path edit mode bin, dann gibt es einen done button, der weiter oben ist als der rest. er sollte etwas weiter unten sein, also auf der gleichen höhe wie die icons
- [x] das bucket tool funktioniert noch nicht ganz richtig, wenn ich den vektor hin und her schiebe, dann flickert es und manchmal ist eine füllfläche sichtbar und manchmal nicht
- [x] make it ease to copy and paste cmyk colors from and to the color picker. All of those should be valid when pasted and this one should be copied: "C:20 M:40 Y:0 K:10"
20, 40, 0, 10
20 40 0 10
20 / 40 / 0 / 10
C20 M40 Y0 K10
C:20 M:40 Y:0 K:10
20% 40% 0% 10%
- [x] in der page overview sind ist die column anzeige bzw. +/- buttons verbuggt, dass auch wenn ich garnicht über den button hovere er trotzdem gehighlighted wird etc. bitte schau da was verbessert werden kann und wie es gefixed werden kann
- [x] in der page overview soll man eine selection haben um mehrere seiten zu markieren. Außerdem soll shift click um alles dazwischen auch zu markieren also 1 ist ausgewählt und shift auf 3 dann werden 1,2,3 ausgewählt. Die ausgewählten seiten soll man performativ nach vorne und nach hinten gezogen werden können, einfach an die position von der die markierten starten sollen. das soll für normale pages und masters gleich funktioniert. Die Seitenzahlen die unter den pages angezeigt werden sollen nicht user-selectable sein. 
- [] height to fixed bevor das autolayout collapsed, weil alle innen fill und außen hug ist mit notification unten, dass gerade bei element ... height/widht auf fixed geändert wurde
- [] wenn mehrere elemente auf einer seite nebeneinander sind (nicht diagonal), also schach analogie wie ein turm, nicht wie ein läufer, dann soll in die richtung bei der sie nebeneinander liegen der abstand zum einen auf der rechten seite im panel stehen als gap. und einmal zwischen den elementen mit der möglichkeit den abstand gleich hier größer und kleiner zu ziehen. also ca. so wie gerade die punkte in den elementen sind, dass jetzt auch zwischen den elementen ein strich ist den man in die richtung ziehen kann in der die nebeneinander liegen und dann den abstand vergrößern oder verringern. Es ist von Figma abgeschaut, da wird das schon gerade gemacht

--- LATER
- [] Figma import with file and or account
- [] overview page for all the personal projekts when opening the webapp, like in figma
- [] inhaltsverzeichnis
- [] datum einfügen wie seitenzahl gerade
- [] conditional text mit variablen
- [] tabellen
- [] verankerte objekte
- [] Linked images: per document, chosen at creation, embed or link. Linking uses File System Access (Chromium) with handles in IndexedDB, so links need a click per session and are missing on other computers; Firefox shows the option disabled with a hint. Preflight reports missing links with relink.
- [] Type as in Figma: axes of variable fonts. krilla 0.8.2 exports them (`Font::new_variable` instances the subset) and harfrust shapes them; CanvasKit 0.42 has no variation binding, so the canvas draws such glyphs as paths from skrifa at the font's location, cached per font, location and glyph.
- [] Place SVG and PDF as vectors, kept as vectors in the exported PDF.
- [] Version history from the Loro history: browse and restore earlier states.
- [] grep styles
- [] add a color wheel inside the color picker for color combinations, like analogous, complementary, etc. 

--- with server (in the future)
- [] make it possible to work together on the same project with link and code share, alot of works needs to be done to make this work good
