import type { ESTree } from 'vite-plus/lint/plugins'

export const called = (node: ESTree.Expression | ESTree.Super, name: string, object?: string) =>
  object === undefined && node.type === 'Identifier'
    ? node.name === name
    : node.type === 'MemberExpression' && !node.computed && node.property.name === name && (object === undefined || (node.object.type === 'Identifier' && node.object.name === object))
