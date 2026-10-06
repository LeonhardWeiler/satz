--- NOW
- [x] inside groups and auto layouts the elemts should have a circle in the middle of them just like when both of them are selected to change position. for autolayouts its neccesesary to move them in the layer order i belive
- [x] settings menu should be centered on the page
- [x] the rulers should also snap the same as any other element
- [x] point 16 is done, but it should also work if the group is selected, right now it only works if an element inside the group is selected, there it should not be the circle. only if the group is selected
- [x] point 25 works, but it should show the smart snap lines with mm, like any other element
- [] komplette liste von funktionen in satz als markdown datei im repo
- [] wrap around für height und width getrennt einstellbar
- [] Performance of `settle` and `lay_out`, which walk the whole tree after every command; at 32 pages the snapshot build takes most of the time.
- [] Kontur innen/außen pro Seite, Absatzlinien oder ‑hintergrund, Textrahmen-Spaltenlinie, Mindesthöhe bei Auto height.
- [x] you should be able to drag select multiple vertices in path edit mode to e.g. delete a bunch of them
- [] when using the bucket tool in path edit mode it should not create new vektors but color the existing one
- [] the bucket tool should only not fill over line crossings. an area is when it is surrounded by lines in all directions, there does not have to be vertices that connect the lines around it. it should work like in indesign
- [x] in the layers panel the names of the elements are faded out to the right, even though they don't need to be truncated. it should only be truncated (faded away) when hovered or hide or lock are selected, because they're the reason for the truncation
- [x] everytime i go into the page overview the layout slighty moves up and then down. i don't know why this happens. investigate and then fix it

--- LATER
1. Figma import with file and or account
5. overview page for all the personal projekts when opening the webapp, like in figma
10. inhaltsverzeichnis
14. datum einfügen wie seitenzahl gerade
15. conditional text mit variablen
16. tabellen
17. verankerte objekte
18. move pages or double pages
20. Linked images: per document, chosen at creation, embed or link. Linking uses File System Access (Chromium) with handles in IndexedDB, so links need a click per session and are missing on other computers; Firefox shows the option disabled with a hint. Preflight reports missing links with relink.
21. Type as in Figma: axes of variable fonts. krilla 0.8.2 exports them (`Font::new_variable` instances the subset) and harfrust shapes them; CanvasKit 0.42 has no variation binding, so the canvas draws such glyphs as paths from skrifa at the font's location, cached per font, location and glyph.
23. Place SVG and PDF as vectors, kept as vectors in the exported PDF.
24. Version history from the Loro history: browse and restore earlier states.
26. grep styles
27. add a color wheel inside the color picker for color combinations, like analogous, complementary, etc. 
28. make it ease to copy and paste cmyk colors from and to the color picker

--- with server (in the future)
1. make it possible to work together on the same project with link and code share, alot of works needs to be done to make this work good
