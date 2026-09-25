import type { Node } from './model'

const PATHS = {
  move: 'M7 4l11 6.5-4.8 1.3 2.9 5-1.7 1-2.9-5L7 16.4z',
  frame: 'M8 3v18M16 3v18M3 8h18M3 16h18',
  rect: 'M5.5 5.5h13v13h-13z',
  ellipse: 'M12 5.5a6.5 6.5 0 1 0 0 13a6.5 6.5 0 1 0 0-13z',
  polygon: 'M12 5.5l7 12.5h-14z',
  star: 'M12 4.5l2.2 4.8 5.3.6-3.9 3.6 1.1 5.2-4.7-2.7-4.7 2.7 1.1-5.2-3.9-3.6 5.3-.6z',
  line: 'M5.5 18.5l13-13',
  arrow: 'M5.5 18.5l13-13M11 5.5h7.5v7.5',
  path: 'M5 18c2-8 5-11 14-13M5 18m-1.5 0a1.5 1.5 0 1 0 3 0a1.5 1.5 0 1 0-3 0',
  pen: 'M12 4l5 8-3 7h-4l-3-7zM12 4v7M12 11m-1 0a1 1 0 1 0 2 0a1 1 0 1 0-2 0',
  text: 'M6 6h12M12 6v13M9.5 19h5',
  group: 'M5.5 5.5h13v13h-13z',
  mask: 'M5.5 5.5h13v13h-13zM12 8.5a3.5 3.5 0 1 0 0 7a3.5 3.5 0 1 0 0-7z',
  masked: 'M8.5 4.5v9h8',
  down: 'M12 5v14M7 14l5 5 5-5',
  right: 'M5 12h14M14 7l5 5-5 5',
  chevron: 'M9 7l5 5-5 5',
  spot: 'M12 6a6 6 0 1 0 0 12a6 6 0 1 0 0-12zM12 11a1 1 0 1 0 0 2a1 1 0 1 0 0-2z',
  process: 'M6 6h12v12h-12zM12 6v12M6 12h12',
  close: 'M7 7l10 10M17 7L7 17',
  plus: 'M12 6v12M6 12h12',
  minus: 'M6 12h12',
  eye: 'M3.5 12s3-5.5 8.5-5.5 8.5 5.5 8.5 5.5-3 5.5-8.5 5.5S3.5 12 3.5 12zM12 12m-2.5 0a2.5 2.5 0 1 0 5 0a2.5 2.5 0 1 0-5 0',
  variable: 'M12 4.5l6.5 3.75v7.5L12 19.5l-6.5-3.75v-7.5z',
  detach: 'M9 15l-2 2a2.8 2.8 0 0 1-4-4l2-2M15 9l2-2a2.8 2.8 0 0 1 4 4l-2 2M8 5v2M5 8h2M16 19v-2M19 16h-2',
  alignLeft: 'M5 6h14M5 10h9M5 14h14M5 18h9',
  alignCenter: 'M5 6h14M7.5 10h9M5 14h14M7.5 18h9',
  alignRight: 'M5 6h14M10 10h9M5 14h14M10 18h9',
  alignJustify: 'M5 6h14M5 10h14M5 14h14M5 18h9',
  eyeOff: 'M3.5 12s3-5.5 8.5-5.5 8.5 5.5 8.5 5.5-3 5.5-8.5 5.5S3.5 12 3.5 12zM5 19L19 5',
} as const

export function Icon({ name, size = 24 }: { name: IconName; size?: number }) {
  const fill = name === 'move'
  return (
    <svg width={size} height={size} viewBox="0 0 24 24" aria-hidden="true">
      <path
        d={PATHS[name]}
        fill={fill ? 'currentColor' : 'none'}
        stroke="currentColor"
        strokeWidth={fill ? 1 : 1.5}
        strokeLinejoin="round"
        strokeDasharray={name === 'group' ? '2.5 2' : undefined}
      />
    </svg>
  )
}

export type IconName = keyof typeof PATHS

export function KindIcon({ node }: { node: Node }) {
  const icon = <Icon name={iconOf(node)} size={16} />
  return node.mask ? <span className="kind" title="Mask">{icon}</span> : icon
}

function iconOf(node: Node): IconName {
  if (node.mask) return 'mask'
  if (node.kind !== 'shape') return node.kind
  if (node.shape !== 'path') return node.shape
  if (node.arrowStart || node.arrowEnd) return 'arrow'
  return node.path.length === 6 ? 'line' : 'path'
}
