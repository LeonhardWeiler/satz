import type { Node } from './model'

const ICONS = {
  move: <path d="M4 2.5 12.5 8 8.5 9 6.5 13z" />,
  frame: <path d="M5 2v12M11 2v12M2 5h12M2 11h12" />,
  rect: <rect x="3" y="3" width="10" height="10" rx=".5" />,
  ellipse: <circle cx="8" cy="8" r="5.5" />,
  polygon: <path d="M8 2.5 13.5 12.5h-11z" />,
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
  pen: (
    <>
      <path d="M8 2 12 8l-2 5.5H6L4 8z" />
      <path d="M8 2v5" />
      <circle cx="8" cy="8.3" r="1" />
    </>
  ),
  text: <path d="M3.5 4V3h9v1M8 3v10M6 13h4" />,
  image: (
    <>
      <rect x="2.5" y="3.5" width="11" height="9" rx="1" />
      <path d="m2.5 11 3.5-3.5 3 3 1.5-1.5 3 3" />
      <circle cx="10.5" cy="6.5" r="1" />
    </>
  ),
  group: <rect x="2.5" y="2.5" width="11" height="11" rx="1" strokeDasharray="2 2" />,
  mask: (
    <>
      <rect x="2.5" y="2.5" width="11" height="11" rx="1" />
      <circle cx="8" cy="8" r="3" />
    </>
  ),
  masked: <path d="M5.5 3v6.5h6" />,
  autoLayout: (
    <>
      <rect x="2.5" y="2.5" width="11" height="4" rx="1" />
      <rect x="2.5" y="9.5" width="11" height="4" rx="1" />
    </>
  ),
  arrowDown: <path d="M8 3v10M4.5 9.5 8 13l3.5-3.5" />,
  arrowRight: <path d="M3 8h10M9.5 4.5 13 8l-3.5 3.5" />,
  arrowLeft: <path d="M13 8H3M6.5 4.5 3 8l3.5 3.5" />,
  plainLine: <path d="M3 8h10" />,
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
  spot: (
    <>
      <circle cx="8" cy="8" r="5.5" />
      <circle cx="8" cy="8" r="1" fill="currentColor" />
    </>
  ),
  process: (
    <>
      <circle cx="8" cy="8" r="5.5" />
      <path d="M8 2.5v11M2.5 8h11" />
    </>
  ),
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
      <rect x="4" y="7" width="8" height="6" rx="1" />
      <path d="M5.5 7V5.5a2.5 2.5 0 0 1 5 0V7" />
    </>
  ),
  unlock: (
    <>
      <rect x="4" y="7" width="8" height="6" rx="1" />
      <path d="M5.5 7V5.5a2.5 2.5 0 0 1 4.8-1" />
    </>
  ),
  variable: <path d="M8 2.5 13 5.25v5.5L8 13.5 3 10.75v-5.5z" />,
  detach: <path d="M6 10 4.5 11.5a2 2 0 0 1-2.8-2.8L3.2 7.2M10 6l1.5-1.5a2 2 0 0 1 2.8 2.8L12.8 8.8M5.5 2.5V4M2.5 5.5H4M10.5 13.5V12M13.5 10.5H12" />,
  alignLeft: <path d="M3 4h10M3 7h6M3 10h10M3 13h6" />,
  alignCenter: <path d="M3 4h10M5 7h6M3 10h10M5 13h6" />,
  alignRight: <path d="M3 4h10M7 7h6M3 10h10M7 13h6" />,
  alignJustify: <path d="M3 4h10M3 7h10M3 10h10M3 13h6" />,
  alignLeftEdges: <path d="M2.5 2v12M5 4.5h8v2.5H5zM5 9h5v2.5H5z" />,
  alignCenters: <path d="M8 2v12M3.5 4.5h9v2.5h-9zM5 9h6v2.5H5z" />,
  alignRightEdges: <path d="M13.5 2v12M3 4.5h8v2.5H3zM6 9h5v2.5H6z" />,
  alignTopEdges: <path d="M2 2.5h12M4.5 5v8h2.5V5zM9 5v5h2.5V5z" />,
  alignMiddles: <path d="M2 8h12M4.5 3.5v9h2.5v-9zM9 5v6h2.5V5z" />,
  alignBottomEdges: <path d="M2 13.5h12M4.5 3v8h2.5V3zM9 6v5h2.5V6z" />,
  distributeX: <path d="M2.5 2v12M13.5 2v12M6.5 5h3v6h-3z" />,
  distributeY: <path d="M2 2.5h12M2 13.5h12M5 6.5h6v3H5z" />,
  tidy: <path d="M3 3h4v4H3zM9 3h4v4H9zM3 9h4v4H3zM9 9h4v4H9z" />,
  alignTop: <path d="M3 3h10M8 5.5v8M5.5 8 8 5.5 10.5 8" />,
  alignMiddle: <path d="M3 8h10M8 2v4M8 10v4M6 4l2 2 2-2M6 12l2-2 2 2" />,
  alignBottom: <path d="M3 13h10M8 2.5v8M5.5 8 8 10.5 10.5 8" />,
  autoWidth: <path d="M2.5 8h11M5 5.5 2.5 8 5 10.5M11 5.5 13.5 8 11 10.5" />,
  autoHeight: <path d="M8 2.5v11M5.5 5 8 2.5 10.5 5M5.5 11 8 13.5 10.5 11" />,
  fixedSize: (
    <>
      <rect x="3" y="3" width="10" height="10" rx=".5" />
      <path d="M6 8h4M8 6v4" />
    </>
  ),
  fontSize: <path d="M2.5 5.5v-2h6.5v2M5.75 3.5v9M9.5 9V8h4v1M11.5 8v4.5" />,
  lineHeight: <path d="M7.5 4h6M7.5 8h6M7.5 12h6M4 3v10M2.5 4.5 4 3l1.5 1.5M2.5 11.5 4 13l1.5-1.5" />,
  letterSpacing: <path d="m3 3 2.25 6.5L7.5 3M8.5 9.5 10.75 3 13 9.5M9.2 7.5h3.1M2.5 12.5h11M4 11l-1.5 1.5L4 14M12 11l1.5 1.5L12 14" />,
  paragraphSpacing: <path d="M2.5 2.5h11M2.5 5h7M2.5 11h11M2.5 13.5h7M12 6.5v3" />,
  insetTop: (
    <>
      <rect x="2.5" y="2.5" width="11" height="11" rx="1.5" strokeOpacity=".45" />
      <path d="M5.5 5.5h5" />
    </>
  ),
  insetRight: (
    <>
      <rect x="2.5" y="2.5" width="11" height="11" rx="1.5" strokeOpacity=".45" />
      <path d="M10.5 5.5v5" />
    </>
  ),
  insetBottom: (
    <>
      <rect x="2.5" y="2.5" width="11" height="11" rx="1.5" strokeOpacity=".45" />
      <path d="M5.5 10.5h5" />
    </>
  ),
  insetLeft: (
    <>
      <rect x="2.5" y="2.5" width="11" height="11" rx="1.5" strokeOpacity=".45" />
      <path d="M5.5 5.5v5" />
    </>
  ),
  columns: (
    <>
      <rect x="2.5" y="3" width="4.5" height="10" rx="1" />
      <rect x="9" y="3" width="4.5" height="10" rx="1" />
    </>
  ),
  gutter: <path d="M3.5 3v10M12.5 3v10M6 8h4M7 7 6 8l1 1M9 7l1 1-1 1" />,
  baselineGrid: <path d="M2.5 4h11M2.5 8h11M2.5 12h11" strokeDasharray="1.5 1.5" />,
  baselineStart: <path d="M2.5 12.5h11M5 2.5v6M3 6.5l2 2 2-2M9 8.5h4.5" />,
  pageNumber: (
    <>
      <rect x="3.5" y="2" width="9" height="12" rx="1.5" />
      <path d="M7.25 6 6.5 11M9.5 6l-.75 5M5.75 7.75h4.5M5.5 9.5h4.5" />
    </>
  ),
  warn: <path d="M8 2.5 14 13H2zM8 6.5v3M8 11.3v.2" />,
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
  doc: <path d="M4 2.5h5.5L12 5v8.5H4zM9.5 2.5V5H12" />,
  portrait: <rect x="4.5" y="2.5" width="7" height="11" rx="1" />,
  landscape: <rect x="2.5" y="4.5" width="11" height="7" rx="1" />,
  opacity: (
    <>
      <circle cx="8" cy="8" r="5.5" />
      <path d="M8 2.5a5.5 5.5 0 0 1 0 11z" fill="currentColor" stroke="none" />
    </>
  ),
  radius: <path d="M3 13V8a5 5 0 0 1 5-5h5" />,
  stroke: (
    <>
      <path d="M3 4h10" />
      <path d="M3 8h10" strokeWidth="2" />
      <path d="M3 12.5h10" strokeWidth="3" />
    </>
  ),
  pages: <path d="M2.5 2.5h4.5v5H2.5zM9 2.5h4.5v5H9zM2.5 9.5h4.5v4H2.5zM9 9.5h4.5v4H9z" />,
  master: <path d="M2.5 4.5h8v9h-8zM5.5 4.5v-2h8v9h-3" />,
  search: (
    <>
      <circle cx="7" cy="7" r="4" />
      <path d="m10 10 3.5 3.5" />
    </>
  ),
  cmd: <path d="M6 6h4v4H6zM6 6H4.5A1.5 1.5 0 1 1 6 4.5zM10 6V4.5A1.5 1.5 0 1 1 11.5 6zM10 10h1.5a1.5 1.5 0 1 1-1.5 1.5zM6 10v1.5A1.5 1.5 0 1 1 4.5 10z" />,
  preflight: <path d="M8 1.8 13.5 4v4c0 3.2-2.4 5.3-5.5 6.2C4.9 13.3 2.5 11.2 2.5 8V4zM5.5 8l1.8 1.8L10.5 6.5" />,
  panelLeft: (
    <>
      <rect x="2" y="3" width="12" height="10" rx="1" />
      <path d="M6 3v10" />
    </>
  ),
  panelRight: (
    <>
      <rect x="2" y="3" width="12" height="10" rx="1" />
      <path d="M10 3v10" />
    </>
  ),
  help: (
    <>
      <circle cx="8" cy="8" r="5.5" />
      <path d="M6.5 6.5a1.5 1.5 0 1 1 2 1.4c-.4.2-.5.5-.5.9v.4M8 10.8v.2" />
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
