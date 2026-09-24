import type { Node } from './model'

const PATHS = {
  move: 'M7 4l11 6.5-4.8 1.3 2.9 5-1.7 1-2.9-5L7 16.4z',
  frame: 'M8 3v18M16 3v18M3 8h18M3 16h18',
  rect: 'M5.5 5.5h13v13h-13z',
  text: 'M6 6h12M12 6v13M9.5 19h5',
  group: 'M5.5 5.5h13v13h-13z',
  chevron: 'M9 7l5 5-5 5',
} as const

export function Icon({ name, size = 24 }: { name: keyof typeof PATHS; size?: number }) {
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

export function KindIcon({ node }: { node: Node }) {
  return <Icon name={node.kind} size={16} />
}
