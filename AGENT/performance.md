# Performance

Native release bench (`engine/src/doc/bench.rs`), median ms, 2026-10-07:

| pages | set_frame | keystroke | pdf  |
| ----- | --------- | --------- | ---- |
| 1     | 0.24      | 0.58      | 47   |
| 8     | 0.59      | 4.93      | 284  |
| 32    | 2.11      | 10.91     | 1075 |

Inks: 86 ms at 36 ppi, 289 ms at 72 ppi. Per command and per drag step the work
grows with the document, not with the change.

`perf` on 32 pages (share of the phase, inclusive):

- set_frame: `flow_all` 60 % (its cache check rereads Loro: `text_frame_of` 35 %,
  spans 20 %), `kind` 17 %, `lay_out` 14 %, `build_snapshot` 12 %,
  `serde_json::to_value` 12 %, `settle` 10 %, `is_node_unexist` 7 %.
- keystroke: `text::set` 59 % (shaping 28 %, Knuth-Plass 9 %, hyphenation 6 %),
  `build_snapshot` 22 % (every page of the story is built again).
- render: `text::draw` 53 % (allocations 16 %, `String` drops 8 %,
  `str::replace` 5 %, `Paint` eq 3 %), `Style::shape` 8 %, encode 5 %.
- snapshot JSON: string escaping 22 %, memmove 20 %, float formatting 15 %.
- pdf: rasterizing 95 % (deflate 33 %, box blur 27 %, fills 21 %).
- inks and png: highp `draw_pixmap` 54–68 %, box blur 13–17 %.
- save: Loro export 59 % (lz4 19 %), the image scan through serde_json 26 %.
- load: `finish` 82 %.
