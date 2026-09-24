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
  chevron: 'M9 7l5 5-5 5',
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
  return <Icon name={iconOf(node)} size={16} />
}

function iconOf(node: Node): IconName {
  if (node.kind !== 'shape') return node.kind
  if (node.shape !== 'path') return node.shape
  if (node.arrowStart || node.arrowEnd) return 'arrow'
  return node.path.length === 6 ? 'line' : 'path'
}
