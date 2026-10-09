import type { Node } from './model'

function aligned(edge: string, ...bars: [number, number, number, number][]) {
  return (
    <>
      <path d={edge} />
      {bars.map(([x, y, width, height]) => (
        <rect key={`${x} ${y}`} x={x} y={y} width={width} height={height} rx="1" fill="currentColor" stroke="none" />
      ))}
    </>
  )
}

const ICONS = {
  move: <path d="M3.6 3.3c-.2-.6.3-1 .8-.7l8.1 4.9c.5.3.4 1-.2 1.1L8.8 9.2l-1.7 3.7c-.3.6-1 .5-1.1-.1z" />,
  frame: <path d="M5 2v12M11 2v12M2 5h12M2 11h12" />,
  rect: <rect x="3" y="3" width="10" height="10" rx="2" />,
  ellipse: <circle cx="8" cy="8" r="5.5" />,
  polygon: <path d="M7.1 3.6a1 1 0 0 1 1.8 0l4.6 8.4a1 1 0 0 1-.9 1.5H3.4a1 1 0 0 1-.9-1.5z" />,
  star: <path d="m8 2.5 1.7 3.6 3.8.5-2.8 2.6.7 3.8L8 11.2 4.6 13l.7-3.8L2.5 6.6l3.8-.5z" />,
  line: <path d="m3 13 10-10" />,
  arrow: <path d="m3 13 10-10M7.5 3H13v5.5" />,
  path: (
    <>
      <path d="M4.5 11.5C5.5 7 8 4.8 11.5 4.5" />
      <circle cx="3.5" cy="12.5" r="1.2" />
      <circle cx="12.5" cy="3.5" r="1.2" />
    </>
  ),
  addPoint: (
    <>
      <path d="M2.5 13.5C4 9.5 6 8 9 7.5" />
      <rect x="1.5" y="12" width="2.5" height="2.5" />
      <path d="M12 2.5v6M9 5.5h6" />
    </>
  ),
  smooth: (
    <>
      <path d="M2.5 12.5C4 6 12 6 13.5 12.5M3 6h10" />
      <circle cx="8" cy="6" r="1.5" />
    </>
  ),
  corner: (
    <>
      <path d="m2.5 13.5 4.6-7.4M8.9 6.1l4.6 7.4" />
      <rect x="6.75" y="3" width="2.5" height="2.5" />
    </>
  ),
  deletePoint: (
    <>
      <path d="M2.5 13.5C4 9.5 6 8 9 7.5" />
      <rect x="1.5" y="12" width="2.5" height="2.5" />
      <path d="M9 5.5h6" />
    </>
  ),
  pen: (
    <>
      <path d="M7.2 2.9a1 1 0 0 1 1.6 0L12 8l-2 5.5H6L4 8z" />
      <path d="M8 2v5" />
      <circle cx="8" cy="8.3" r="1" />
    </>
  ),
  text: <path d="M3.5 4V3h9v1M8 3v10M6 13h4" />,
  image: (
    <>
      <rect x="2.5" y="3.5" width="11" height="9" rx="2" />
      <path d="m2.5 11 3.5-3.5 3 3 1.5-1.5 3 3" />
      <circle cx="10.5" cy="6.5" r="1" />
    </>
  ),
  bucket: (
    <>
      <path d="M7 1.5l6 6-5 5-6-6zM2 6.5h11" />
      <path d="M14 10.5c.8 1.2 1 1.8 1 2.2a1 1 0 0 1-2 0c0-.4.2-1 1-2.2z" />
    </>
  ),
  eyedropper: <path d="M10.5 2.5l3 3M9 4l3 3-6.5 6.5H3.5V11.5z" />,
  fit: <path d="M1.5 5.5v-4h4M10.5 1.5h4v4M14.5 10.5v4h-4M5.5 14.5h-4v-4" />,
  crop: <path d="M4.5 1.5v10h10M1.5 4.5h10v10" />,
  adjust: (
    <>
      <path d="M2.5 5h2M7.5 5h6M2.5 11h6M11.5 11h2" />
      <circle cx="6" cy="5" r="1.5" />
      <circle cx="10" cy="11" r="1.5" />
    </>
  ),
  group: <rect x="2.5" y="2.5" width="11" height="11" rx="2" strokeDasharray="2 2" />,
  mask: (
    <>
      <rect x="2.5" y="2.5" width="11" height="11" rx="2" />
      <circle cx="8" cy="8" r="3" />
    </>
  ),
  masked: <path d="M5.5 3v6.5h6" />,
  autoLayout: (
    <>
      <rect x="2.5" y="2.5" width="11" height="4" rx="1.5" />
      <rect x="2.5" y="9.5" width="11" height="4" rx="1.5" />
    </>
  ),
  flatten: (
    <>
      <path d="M8 3.5 12.5 12h-9z" />
      <rect x="6.5" y="2" width="3" height="3" rx=".5" fill="currentColor" />
      <rect x="2" y="10.5" width="3" height="3" rx=".5" fill="currentColor" />
      <rect x="11" y="10.5" width="3" height="3" rx=".5" fill="currentColor" />
    </>
  ),
  flipHorizontal: <path d="M8 2v12M6 4.5v8H2.5zM10 4.5v8h3.5z" />,
  flipVertical: <path d="M2 8h12M4.5 6h8V2.5zM4.5 10h8v3.5z" />,
  reset: <path d="M3 3v3h3M3.4 6A5 5 0 1 1 3 8.5" />,
  ratio: <path d="M7 9l2-2M6.5 5.5l1-1a2.5 2.5 0 0 1 3.5 3.5l-1 1M9.5 10.5l-1 1a2.5 2.5 0 0 1-3.5-3.5l1-1" />,
  arrowDown: <path d="M8 3v10M4.5 9.5 8 13l3.5-3.5" />,
  arrowUp: <path d="M8 13V3M4.5 6.5 8 3l3.5 3.5" />,
  arrowRight: <path d="M3 8h10M9.5 4.5 13 8l-3.5 3.5" />,
  arrowLeft: <path d="M13 8H3M6.5 4.5 3 8l3.5 3.5" />,
  plainLine: <path d="M3 8h10" />,
  strokeWeight: (
    <>
      <path d="M3 3.5h10" strokeWidth="0.75" />
      <path d="M3 7.5h10" strokeWidth="1.5" />
      <path d="M3 12h10" strokeWidth="2.5" />
    </>
  ),
  strokeInside: (
    <>
      <rect x="2.5" y="8" width="11" height="5" fill="currentColor" stroke="none" opacity=".5" />
      <path d="M1.5 8h13" />
    </>
  ),
  strokeCenter: (
    <>
      <rect x="2.5" y="5.5" width="11" height="5" fill="currentColor" stroke="none" opacity=".5" />
      <path d="M1.5 8h13" />
    </>
  ),
  strokeOutside: (
    <>
      <rect x="2.5" y="3" width="11" height="5" fill="currentColor" stroke="none" opacity=".5" />
      <path d="M1.5 8h13" />
    </>
  ),
  joinMiter: <path d="M3.5 13V3.5H13" />,
  joinRound: <path d="M3.5 13V8a4.5 4.5 0 0 1 4.5-4.5h5" />,
  joinBevel: <path d="M3.5 13V7L7 3.5h6" />,
  capNone: <path d="M13 5H6v6h7" />,
  capRound: <path d="M13 5H6a3 3 0 0 0 0 6h7" />,
  capSquare: <path d="M13 5H3v6h10M6 7v2" />,
  chevron: <path d="m6.5 4 4 4-4 4" />,
  left: <path d="m9.5 4-4 4 4 4" />,
  close: <path d="m4 4 8 8M12 4l-8 8" />,
  check: <path d="m3.5 8.5 3 3 6-7" />,
  plus: <path d="M8 4.5v7M4.5 8h7" />,
  minus: <path d="M4.5 8h7" />,
  eye: (
    <>
      <path d="M1.5 8S4 3.5 8 3.5 14.5 8 14.5 8 12 12.5 8 12.5 1.5 8 1.5 8z" />
      <circle cx="8" cy="8" r="2" />
    </>
  ),
  eyeOff: (
    <path d="m2.5 2.5 11 11M6.2 4A6.5 6.5 0 0 1 8 3.5c4 0 6.5 4.5 6.5 4.5a11 11 0 0 1-1.8 2.3M10 11.9a5.6 5.6 0 0 1-2 .6C4 12.5 1.5 8 1.5 8a11 11 0 0 1 2.4-2.8" />
  ),
  lock: (
    <>
      <rect x="4" y="7" width="8" height="6" rx="1.5" />
      <path d="M5.5 7V5.5a2.5 2.5 0 0 1 5 0V7" />
    </>
  ),
  unlock: (
    <>
      <rect x="4" y="7" width="8" height="6" rx="1.5" />
      <path d="M5.5 7V5.5a2.5 2.5 0 0 1 4.8-1" />
    </>
  ),
  variable: <path d="M8 2.5 13 5.25v5.5L8 13.5 3 10.75v-5.5z" />,
  detach: <path d="M6.5 9.5 5 11a2.1 2.1 0 0 1-3-3l1.5-1.5M9.5 6.5 11 5a2.1 2.1 0 0 1 3 3l-1.5 1.5M3 3l10 10" />,
  alignLeft: <path d="M3 4h10M3 7h6M3 10h10M3 13h6" />,
  alignCenter: <path d="M3 4h10M5 7h6M3 10h10M5 13h6" />,
  alignRight: <path d="M3 4h10M7 7h6M3 10h10M7 13h6" />,
  alignJustify: <path d="M3 4h10M3 7h10M3 10h10M3 13h6" />,
  alignLeftEdges: aligned('M2.5 2v12', [5, 4, 8, 3], [5, 9, 5, 3]),
  alignCenters: aligned('M8 2v12', [3, 4, 10, 3], [5, 9, 6, 3]),
  alignRightEdges: aligned('M13.5 2v12', [3, 4, 8, 3], [6, 9, 5, 3]),
  alignTopEdges: aligned('M2 2.5h12', [4, 5, 3, 8], [9, 5, 3, 5]),
  alignMiddles: aligned('M2 8h12', [4, 3, 3, 10], [9, 5, 3, 6]),
  alignBottomEdges: aligned('M2 13.5h12', [4, 3, 3, 8], [9, 6, 3, 5]),
  distributeX: aligned('M2.5 2v12M13.5 2v12', [6.5, 4, 3, 8]),
  distributeY: aligned('M2 2.5h12M2 13.5h12', [4, 6.5, 8, 3]),
  union: <path d="M3 3h7v3h3v7H6v-3H3z" />,
  subtract: (
    <>
      <path d="M3 3h7v3H6v4H3z" fill="currentColor" />
      <rect x="6" y="6" width="7" height="7" />
    </>
  ),
  intersect: (
    <>
      <rect x="3" y="3" width="7" height="7" />
      <rect x="6" y="6" width="7" height="7" />
      <rect x="6" y="6" width="4" height="4" fill="currentColor" />
    </>
  ),
  exclude: <path d="M3 3h7v7H3zM6 6h7v7H6z" fill="currentColor" fillRule="evenodd" />,
  tidy: aligned('', [3, 3, 4, 4], [9, 3, 4, 4], [3, 9, 4, 4], [9, 9, 4, 4]),
  alignTop: <path d="M3 3h10M8 5.5v8M5.5 8 8 5.5 10.5 8" />,
  alignMiddle: <path d="M3 8h10M8 2v4M8 10v4M6 4l2 2 2-2M6 12l2-2 2 2" />,
  alignBottom: <path d="M3 13h10M8 2.5v8M5.5 8 8 10.5 10.5 8" />,
  autoWidth: <path d="M2.5 8h11M5 5.5 2.5 8 5 10.5M11 5.5 13.5 8 11 10.5" />,
  autoHeight: <path d="M8 2.5v11M5.5 5 8 2.5 10.5 5M5.5 11 8 13.5 10.5 11" />,
  autoFit: <path d="M2.5 6V2.5H6M10 2.5h3.5V6M13.5 10v3.5H10M6 13.5H2.5V10" />,
  fixedSize: (
    <>
      <rect x="3" y="3" width="10" height="10" rx="2" />
      <path d="M6 8h4M8 6v4" />
    </>
  ),
  fontSize: <path d="M2.5 5.5v-2h6.5v2M5.75 3.5v9M9.5 9V8h4v1M11.5 8v4.5" />,
  lineHeight: <path d="M7.5 4h6M7.5 8h6M7.5 12h6M4 3v10M2.5 4.5 4 3l1.5 1.5M2.5 11.5 4 13l1.5-1.5" />,
  letterSpacing: <path d="m3 3 2.25 6.5L7.5 3M8.5 9.5 10.75 3 13 9.5M9.2 7.5h3.1M2.5 12.5h11M4 11l-1.5 1.5L4 14M12 11l1.5 1.5L12 14" />,
  paragraphSpacing: <path d="M2.5 2.5h11M2.5 5h7M2.5 11h11M2.5 13.5h7M12 6.5v3" />,
  more: <path d="M3.5 8h.01M8 8h.01M12.5 8h.01" strokeWidth="2.5" />,
  columns: (
    <>
      <rect x="2.5" y="3" width="4.5" height="10" rx="1" />
      <rect x="9" y="3" width="4.5" height="10" rx="1" />
    </>
  ),
  omega: <path d="M3 13h3v-1.6a4.5 4.5 0 1 1 4 0V13h3" />,
  warn: <path d="M7.1 3.1a1 1 0 0 1 1.8 0l5.2 9.4a1 1 0 0 1-.9 1.5H2.8a1 1 0 0 1-.9-1.5zM8 6.5v3M8 11.3v.2" />,
  error: (
    <>
      <circle cx="8" cy="8" r="5.5" />
      <path d="M8 5v3.5M8 10.8v.2" />
    </>
  ),
  ok: (
    <>
      <circle cx="8" cy="8" r="5.5" />
      <path d="m5.5 8 1.8 1.8L10.8 6" />
    </>
  ),
  folder: <path d="M2.5 4.5a1 1 0 0 1 1-1h3l1.5 1.5h4.5a1 1 0 0 1 1 1v6a1 1 0 0 1-1 1h-9a1 1 0 0 1-1-1z" />,
  doc: <path d="M5.5 2.5h4L12 5v7.5a1.5 1.5 0 0 1-1.5 1.5h-5A1.5 1.5 0 0 1 4 12.5V4a1.5 1.5 0 0 1 1.5-1.5zM9.5 2.5V5H12" />,
  portrait: <rect x="4.5" y="2.5" width="7" height="11" rx="1.5" />,
  landscape: <rect x="2.5" y="4.5" width="11" height="7" rx="1.5" />,
  opacity: (
    <>
      <circle cx="8" cy="8" r="5.5" />
      <path d="M8 2.5a5.5 5.5 0 0 1 0 11z" fill="currentColor" stroke="none" />
    </>
  ),
  sides: <path d="M5 2.5h6M13.5 5v6M5 13.5h6M2.5 5v6" />,
  radius: <path d="M3 13V8a5 5 0 0 1 5-5h5" />,
  stroke: (
    <>
      <path d="M3 4h10" />
      <path d="M3 8h10" strokeWidth="2" />
      <path d="M3 12.5h10" strokeWidth="3" />
    </>
  ),
  pages: (
    <>
      <rect x="2.5" y="2.5" width="4.5" height="5" rx="1" />
      <rect x="9" y="2.5" width="4.5" height="5" rx="1" />
      <rect x="2.5" y="9.5" width="4.5" height="4" rx="1" />
      <rect x="9" y="9.5" width="4.5" height="4" rx="1" />
    </>
  ),
  master: <path d="M2.5 4.5h8v9h-8zM5.5 4.5v-2h8v9h-3" />,
  search: (
    <>
      <circle cx="7" cy="7" r="4" />
      <path d="m10 10 3.5 3.5" />
    </>
  ),
  preflight: <path d="M8 1.8 13.5 4v4c0 3.2-2.4 5.3-5.5 6.2C4.9 13.3 2.5 11.2 2.5 8V4zM5.5 8l1.8 1.8L10.5 6.5" />,
  panelLeft: (
    <>
      <rect x="2" y="3" width="12" height="10" rx="2" />
      <path d="M6 3v10" />
    </>
  ),
  panelRight: (
    <>
      <rect x="2" y="3" width="12" height="10" rx="2" />
      <path d="M10 3v10" />
    </>
  ),
  help: (
    <>
      <circle cx="8" cy="8" r="5.5" />
      <path d="M6.5 6.5a1.5 1.5 0 1 1 2 1.4c-.4.2-.5.5-.5.9v.4M8 10.8v.2" />
    </>
  ),
  settings: (
    <>
      <circle cx="8" cy="8" r="2" />
      <path d="M12.5 6.64 13.88 6.81v2.38l-1.38.17-.36.86.86 1.1L11.32 13l-1.1-.86-.86.36-.17 1.38H6.81l-.17-1.38-.86-.36-1.1.86L3 11.32l.86-1.1-.36-.86-1.38-.17V6.81l1.38-.17.36-.86L3 4.68 4.68 3l1.1.86.86-.36.17-1.38h2.38l.17 1.38.86.36 1.1-.86L13 4.68l-.86 1.1z" />
    </>
  ),
}

export type IconName = keyof typeof ICONS

export function Icon({ name, size = 16 }: { name: IconName; size?: number }) {
  return (
    <svg
      className="icon"
      width={size}
      height={size}
      viewBox="0 0 16 16"
      fill="none"
      stroke="currentColor"
      strokeWidth="1.25"
      strokeLinecap="round"
      strokeLinejoin="round"
      aria-hidden="true"
    >
      {ICONS[name]}
    </svg>
  )
}

export function KindIcon({ node }: { node: Node }) {
  const icon = <Icon name={iconOf(node)} />
  return node.mask ? <span className="kind" title="Mask">{icon}</span> : icon
}

function iconOf(node: Node): IconName {
  if (node.mask) return 'mask'
  if (node.kind === 'frame' && node.direction !== 'none') return 'autoLayout'
  if (node.kind !== 'shape') return node.kind
  if (node.shape !== 'path') return node.shape
  if (node.arrowStart || node.arrowEnd) return 'arrow'
  return node.path.length === 6 ? 'line' : 'path'
}
