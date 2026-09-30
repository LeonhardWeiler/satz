20. man soll wenn man double clickt und dann auf die seite zieht horizontal bzw. vertikale rouler machen an den dann die items snappen können
21. in dem path edit mode, soll man mit ctrl + klick die bezier kurven entweder hinzufügen oder entfernen können, für beide soll es jeweils einen button auch in der toolbar geben
23. texte sollen auch eine stroke bekommen, die um den text herum geht
25. wenn ich von rgb auf cmyk umwandle, sollen alle farben sofort mit umgewandelt werden und dann natürlich nicht im preflight angezgeit werden
26. wenn man mehrere elemente auswählt, dann sollen die align optionen oben in der leiste angezeigt werden über den elementen
27. es ist immer noch so, dass wenn ich auf ein z.b. r halte, dass dann mein cursor weg ist
29. wenn man außerhalb der page mit einem tool hinklickt soll der normale cursor (v) ausgewählt werden
31. short of bleed soll auch auf der ersten seite angezeigt werden
32. wenn ich ctrl + clicke und weit genug mit der maus weg bin, soll es unter dem cursor in der mitte positioniert werden
33. wenn ich bei einem text element ctrl + v clicke, soll der text in dem textfeld ins clipboard kopiert werden
34. es soll einen keyboard shortcut für auto width und auto height bei text geben
35. wenn ich ein rechteck, darunter mit abstand einen text und darunter mit abstand wieder einen rechteck hab, soll der abstand über dem element auch angezeigt (smart) werden, dass ich sehe, wann es gleich ist und dann darauf auch snappen
36. 2 von den entfernungen in mm zu anderen elementen sollen nie übereinander angezeigt werden, ich hab gerade einen fall gehalb wo dann nur eines sichtbar war, bis ich ein element ein bisschen bewegt hab, dass sie nicht mehr overlappen, es soll immer so positioniert werden, dass man alles sehen kann
37. die selection color soll an das farbschema angepasst sein
38. in der command pallette sollen mehr sinnvolle optionen hinzugefügt werden
39. wenn man in einer gruppe autolayout oder selection von mehreren elementen ist, soll ein kreis in der mitte der elemente sichtbar sein bei dem man durch hin und her ziehen deren position tauschen kann wie in figma
40. implement all the points under later in TODO.md
41. überlege dir für alle elemente noch einmal tief und schlau welche optionen alle sinnvoll und praktisch wären und implementiere sie
42. überlege dir features die noch gemacht werden sollen und schreibe sie unter later in TODO.md
43. Mach einene ausführlichen performance audit sowie einen audit der bundle size und der ci time. schreibe deine findings in die jeweiligen dateien unter AGENT
44. wenn ich im text doppelklicke und dann ziehe, dann soll es nicht das wort markieren sondern die section machen, also der double click wird ignoriert, wenn ich ziehe
48. resize swatch thing horizontally
51. auto fit (auto width when theres a single line, auto height when theres multiple lines) as an additional option for a text frame and when double clicking on the corner of a text frame and use this as the keyboardshortcut
52. add a plus icon when you're on the last page and want to add another instead of a disabled arrow (page up/down should only go to existing pages)
53. make it possible to rotate a line, path, etc. over its center
54. when drawing a line it should not show the rectangle bounding box, only the real line
55. use as mask works for elements above, but it should work for elements below
56. i cannot drag elements that are not visible because they are in a masked and are outside of that, i can only drag where the element is visible
57. on macos (firefox) the text often is faded out to the right (because of the possiblity of an overflow) that dont actually overflow. on chromium i don't have that problem
59. add options for basic image editing, like crop (also when resizing with ctrl), contrast, brightness, etc. they should appear in the bar above the element
60. images that i selected to import and now am placing (only with the outline) are not snapped like an existing element
62. when zooming in/out, the elements should not move but only scale
64. when typing lorem in a textbox and then it tab it should automatically add a 10 word lorem ipsum text if you write lorem100 it should automatically add a 100 word lorem ipsum text, etc.
66. auto height when double clicking on the top or bottom of a textbox and auto widht when clicking on the left or right of a textbox
72. show icons next to the specifig font options, like ligatures to make it easier to understand like figma does
73. make it possible to make boolean operations on things like rectangles, triangles, ... (not text unless its made into an vector). the buttons/icons should be below the align buttons
74. make it possiblel to convert text to vectors
75. when in vector editing mode, you should be able to fill each area seperately with a bucket tool
76. make it easier to grab the border radius handle at the corners (more space to click and drag)
77. when switching between ctrl + \ the page should be recentererd/sized with shift + 1
78. the bounding box should include the stroke
79. add a settings menu to change the unit from mm to in for example, keyboard shortcuts, ... and the language and ui in the future. it should be saved to local storage
80. change, keyboard shortcuts, should be changed from the shortcut menu
81. ctrl + and ctrl - should work a bit more subtle
82. when right clicking onto a swatch color it should show options for that like, duplicate, delete, rename, etc.
83. in the settings the option to disable the toolbar above elements should be added
84. when right clicking on the canvas you should be able to un/hide the rulers
85. when an element is selected and you right click somewhere else, like the toolbar, the selection should be gone and it should show the correct menu
86. when changing from solid to linear gradient it should not change the visualisation of the color, because now its bigger and has a smaller border radius
87. at the toolbar above the element it should show if there is a gradient and not show, that there is now fill, gradients can only be created from the real menu on the right
88. in the settings it should be changable if you see the elements on the left only for one page or the double page, but it should then say in the element selectino thing a small collapsable heading for one page and the other
89. lines should have the option to be squiggly, etc. 
90. you should be able to hold alt on the toolbar above the element when dragging, right now the toolbar then dissapears because it things you want to see the distance to other elements
91. when there are multiple elements selected in the layer panel  there is no margin between them and it looks of, i think the padding can be reduced slightly and a small margin added
92. the arrow down icon in the toolbar to select more primitive objects has a to little padding on the sides, but it should not be further away
93. mach eine ausführliche analyse von anderen programmen, wie figma, indesign, scribus, ... und schau welche funktionen sie bieten, die ich hier noch nicht implementiert habe, schreib das unter AGENT in eine comparison.md datei
94. when ich den preflight editor offen habe und ein element, das als outside the gamut markiert ist bewege, dann dauert es ca. 1,5s bis die selection geupdated ist, schau dass das schneller funktioniert
95. lass die ganzen cmyk sachen im preflight weg, wenn die datei sowieso rgb ist
96. bringe raster in die gleiche zeile wie facing pages gerade ist und integriere die checkbox besser ins layout

--- LATER
1. Figma import with file and or account
2. InDesign import with file
3. Imports from other Design programs
4. geh jeden part des designs ausführlich durch und schau genau wo es inkonsitezen gibt. es soll überall einheitlich aussehen, schau das auch bei den icons, dass die dazu passen, etc.
5. overview page for all the personal projekts when opening the webapp, like in figma
6. gute verbindung mit ki, dass agents wie claude oder gpt einfach und praktisch damit arbeiten können. Schau bei figma wie es gut funktioniern könnte
7. man soll teile der website auswählen können und in diesem bereich die ki prompten, z.b. dass dieser bereich redesigned werden soll

--- with server (in the future)
1. make it possible to work together on the same project with link and code share, alot of works needs to be done to make this work good
