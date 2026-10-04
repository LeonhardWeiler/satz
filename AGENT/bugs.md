--- NOW
1. in crop mode the snapping works, but not when holding down ctrl
3. [x] add a gear icon next to the keyboard shortcuts icon for the settings and move both of them left to the preflight icon
4. [x] add the same amount of padding to the preflight icon on the left and the right
6. [x] in the page overview pages section the "bar" at the top with "Pages" and columns, ... schould also stay at the exact same position when scrolling horizontally
7. [x] the master pages should not have left and right pages if they are the same in the layers overview. that means that at the top left of the Canvas e.g. A-Master L R it should just say A-Master
8. include the reset overrides button when changing the master layout with ctrl + shift better into the layout, so it looks better on the panel on the right
9. include the Keep aspect ratio button better into the layout
10. does text wrap work and if yes how, because i wasn't able to figure it out, maybe it should be more intuitive
11. the rename master right click menu thing doesn't work in the page overview
12. variables should have the same right click and hover controls as swatches (right click to rename, delete, edit, duplicate), hover to delete on the right
13. don't show the drag cursor on non draggable controls like names etc.
14. [x] show the page thing that currently is in the top left of the canvas in the top center of the page in the same row as the toolbar and export, etc.
15. groups should have a quickbar above the selection to align the items inside the group, make it visually clear that its aligning the items inside and not the group itself
16. inside groups and auto layouts the elemts should have a circle in the middle of them just like when both of them are selected to change position. for autolayouts its neccesesary to move them in the layer order i belive
17. when editing the name of the file in the top left, it should still show the extension (even though it can't be edited) and the text should stay in the same position when editing compared to when not editing
18. [x] the arrow down icon dropdown for the element primitives selection has to little margin to the pen tool to its right
20. flatten, flip horizontally, flip vertically and add auto layout should be accesible through icons and not just the context menu or a keyboard shortcut
21. make the canvas not go to the side indefinitely (+/- 20000mm?)
22. have presets for layout grids
23. when keep aspect ratio is ticked on, i cannot resize the element smaller than it was then in the canvas (on the controls on the right it works correctly).
24. [x] open the "New Document" dialog in the center of the page
25. the rulers should also snap the same as any other element

--- LATER
1. Figma import with file and or account
5. overview page for all the personal projekts when opening the webapp, like in figma
10. inhaltsverzeichnis
13. eine markdown datei mit allen funktionen in satz
14. datum einfügen wie seitenzahl gerade
15. conditional text mit variablen
16. tabellen
17. verankerte objekte
18. move pages or double pages
19. komplette liste von funktionen in satz als markdown datei im repo
20. Linked images: per document, chosen at creation, embed or link. Linking uses File System Access (Chromium) with handles in IndexedDB, so links need a click per session and are missing on other computers; Firefox shows the option disabled with a hint. Preflight reports missing links with relink.
21. Type as in Figma: axes of variable fonts (krilla 0.8.2 and CanvasKit drawGlyphs cannot draw variations yet).
22. Performance of `settle` and `lay_out`, which walk the whole tree after every command; at 32 pages the snapshot build takes most of the time.
23. Place SVG and PDF as vectors, kept as vectors in the exported PDF.
24. Version history from the Loro history: browse and restore earlier states.
25. Kontur innen/außen pro Seite, Absatzlinien oder ‑hintergrund, Textrahmen-Spaltenlinie, Mindesthöhe bei Auto height.
26. grep styles
27. add a color wheel inside the color picker for color combinations, like analogous, complementary, etc. 
28. make it ease to copy and paste cmyk colors from and to the color picker
29. when using the bucket tool in path edit mode it should not create new vektors but color the existing one
30. the bucket tool should only not fill over line crossings. an area is when it is surrounded by lines in all directions, there does not have to be vertices that connect the lines around it. it should work like in indesign
31. you should be able to drag select multiple vertices in path edit mode to e.g. delete a bunch of them

--- with server (in the future)
1. make it possible to work together on the same project with link and code share, alot of works needs to be done to make this work good
