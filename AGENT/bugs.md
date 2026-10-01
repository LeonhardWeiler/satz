2. man soll wenn man double clickt und dann auf die seite zieht horizontal bzw. vertikale rouler machen an den dann die items snappen können
3. in dem path edit mode, soll man mit ctrl + klick die bezier kurven entweder hinzufügen oder entfernen können, für beide soll es jeweils einen button auch in der toolbar geben
4. texte sollen auch eine stroke bekommen, die um den text herum geht
5. wenn ich von rgb auf cmyk umwandle, sollen alle farben sofort mit umgewandelt werden und dann natürlich nicht im preflight angezgeit werden
7. es ist immer noch so, dass wenn ich auf ein z.b. r halte, dass dann mein cursor weg ist
15. implement all the points under later in TODO.md
16. überlege dir für alle elemente noch einmal tief und schlau welche optionen alle sinnvoll und praktisch wären und implementiere sie
17. überlege dir features die noch gemacht werden sollen und schreibe sie unter later in TODO.md
18. Mach einene ausführlichen performance audit sowie einen audit der bundle size und der ci time. schreibe deine findings in die jeweiligen dateien unter AGENT
19. in der page overview soll man die vertikale master linie nach links und rechts ziehen können, die vorschaubilder von den master seiten sollen dann auch mitgescaled werden
20. in der page overview sollen die pages (nicht master) etwas größer angezeigt werden und man soll rein und rauszoomen können, aber nicht ganz so nah, und man soll auch die option haben ob man mehrere spalten haben will in denen die pages angezeigt werden
23. resize swatch thing horizontally
24. auch wenn man ein element ausgewählt hat, soll man immer mit single click bei den swatch colors diese ändern können, wenn auf die swatch colors links unten klickt, sollen sie nie gesetzt werden, dafür gibt es die controls auf der rechten seite oder die toolbar über dem element
25. auto fit (auto width when theres a single line, auto height when theres multiple lines) as an additional option for a text frame and when double clicking on the corner of a text frame and use this as the keyboardshortcut
27. when drawing a line it should show the selection like it shows when it is selected later
28. use as mask works for elements above, but it should work for elements below
29. i cannot drag elements that are not visible because they are in a masked and are outside of that, i can only drag where the element is visible
30. on macos (firefox) the text often is faded out to the right (because of the possiblity of an overflow) that dont actually overflow. on chromium i don't have that problem
31. add options for basic image editing, like crop (also when resizing with ctrl), contrast, brightness, etc. they should appear in the bar above the element
32. what does absolute position for an element inside an auto layout mean, is it necesesary/helpful. it should not change the position when checked
33. das text style dropdown soll gleich aussehen wie alle anderen dropdowns
34. when zooming in/out, the elements should not move but only scale
36. auto height when double clicking on the top or bottom of a textbox and auto widht when clicking on the left or right of a textbox
37. show icons next to the specifig font options, like ligatures to make it easier to understand like figma does
38. make it possible to make boolean operations on things like rectangles, triangles, ... (not text unless its made into an vector). the buttons/icons should be below the align buttons
39. make it possiblel to convert text to vectors
40. when in vector editing mode, you should be able to fill each area seperately with a bucket tool
41. when switching between ctrl + \ the page should be recentererd/sized with shift + 1, it works but you can see like a transition of the canvas. the canvas should stay exactly the same, also the same happening with alt + 1. maybe there is a smarter way then shift + 1 so that the panels on top aren't moving the canvas
42. the bounding box should include the stroke
43. add a settings menu to change the unit from mm to in for example, ... and the language and ui in the future. it should be saved to local storage
44. change, keyboard shortcuts, should be changed from the shortcut menu
45. in the settings the option to disable the toolbar above elements should be added
46. when un/hiding the rulers the layout ushow not change, the canvas should stay exactly the same its the same thing as with the panels with alt + 1 or ctrl + \
47. when changing from solid to linear gradient with one of the gradients selected the layout moves up a bit, it should stay the same
49. in the settings it should be changable if you see the elements on the left only for one page or the double page, but it should then say in the element selectino thing a small collapsable heading for one page and the other
50. lines should have the option to be squiggly, etc. 
51. mach eine ausführliche analyse von anderen programmen, wie figma, indesign, scribus, ... und schau welche funktionen sie bieten, die ich hier noch nicht implementiert habe, schreib das unter AGENT in eine comparison.md datei
52. when ich den preflight editor offen habe und ein element, das als outside the gamut markiert ist bewege, dann dauert es ca. 1,5s bis die selection geupdated ist, schau dass das schneller funktioniert
54. bei den Document Settings sieht man es besonders, aber es ist an vielen orten der Fall. Das dropdown menü ist breiter als die andern menüs und wieder anders sieht das landscape oder portrait mode selector aus. Alle Settings sollen genau gleich aussehen. Damit meine ich, gleiche höhe, breite, border radius, padding, margin, etc.
55. when the line is selected when a selection is dragged, only the line should be highlighted, like when clicked, not the whole bounding box

--- LATER
1. Figma import with file and or account
2. InDesign import with file
3. Imports from other Design programs
4. geh jeden part des designs ausführlich durch und schau genau wo es inkonsitezen gibt. es soll überall einheitlich aussehen, schau das auch bei den icons, dass die dazu passen, etc.
5. overview page for all the personal projekts when opening the webapp, like in figma
7. man soll teile der website auswählen können und in diesem bereich die ki prompten, z.b. dass dieser bereich redesigned werden soll
10. inhaltsverzeichnis
13. eine markdown datei mit allen funktionen in satz
14. datum einfügen wie seitenzahl gerade
15. conditional text mit variablen
16. tabellen
17. verankerte objekte
18. move pages or double pages
19. flip horizontally and vertically

--- with server (in the future)
1. make it possible to work together on the same project with link and code share, alot of works needs to be done to make this work good
