import { defineRule, type ESTree } from 'vite-plus/lint/plugins'
import { called } from './match.ts'

const url = 'A file that calls URL.createObjectURL also calls URL.revokeObjectURL.'
const engine = 'An Engine made here is freed in a finally of the same function.'
const cancel = 'A file that listens to pointerup also listens to pointercancel or lostpointercapture.'
const remove = 'A listener added here is removed in the same file, or passes once: true or a signal.'
const ends = ['pointercancel', 'lostpointercapture']
const event = (n: ESTree.CallExpression) => (n.arguments[0]?.type === 'Literal' && typeof n.arguments[0].value === 'string' ? n.arguments[0].value : undefined)
const once = (n: ESTree.CallExpression) => {
  const o = n.arguments[2]
  return o?.type === 'ObjectExpression' && o.properties.some((p) => p.type === 'Property' && p.key.type === 'Identifier' && (p.key.name === 'signal' || (p.key.name === 'once' && p.value.type === 'Literal' && p.value.value === true)))
}

const scope = (n: ESTree.Node) => {
  let s = n.parent
  while (s && s.type !== 'FunctionDeclaration' && s.type !== 'FunctionExpression' && s.type !== 'ArrowFunctionExpression') s = s.parent
  return s
}
const inFinally = (n: ESTree.Node) => {
  for (let c: ESTree.Node = n, p = n.parent; p && p !== scope(n); c = p, p = p.parent) if (p.type === 'TryStatement' && p.finalizer === c) return true
  return false
}

export default defineRule({
  create: (context) => {
    const made: ESTree.Node[] = []
    let revoked = false
    const engines: [ESTree.Node, string | undefined, ESTree.Node | null][] = []
    const freed: [string, ESTree.Node | null][] = []
    const ups: ESTree.Node[] = []
    let cancelled = false
    const added: [ESTree.Node, string][] = []
    const removed = new Set<string>()
    const listen = (node: ESTree.Node, event: string) => {
      if (event === 'pointerup') ups.push(node)
      if (ends.includes(event)) cancelled = true
    }
    const make = (node: ESTree.Node) => {
      let d = node.parent
      while (d?.type === 'ConditionalExpression') d = d.parent
      engines.push([node, d?.type === 'VariableDeclarator' && d.id.type === 'Identifier' ? d.id.name : undefined, scope(node)])
    }
    return {
      NewExpression(node) {
        if (called(node.callee, 'Engine')) make(node)
      },
      JSXAttribute(node) {
        if (node.name.type === 'JSXIdentifier' && node.name.name.startsWith('on')) listen(node, node.name.name.slice(2).toLowerCase())
      },
      CallExpression(node) {
        const e = event(node)
        if (e && called(node.callee, 'addEventListener')) {
          listen(node, e)
          if (!once(node)) added.push([node, e])
        }
        if (e && called(node.callee, 'removeEventListener')) removed.add(e)
        if (called(node.callee, 'createObjectURL', 'URL')) made.push(node)
        if (called(node.callee, 'revokeObjectURL', 'URL')) revoked = true
        if (called(node.callee, 'sample', 'Engine') || called(node.callee, 'blank', 'Engine')) make(node)
        if (called(node.callee, 'free') && node.callee.type === 'MemberExpression' && node.callee.object.type === 'Identifier' && inFinally(node)) freed.push([node.callee.object.name, scope(node)])
      },
      'Program:exit'() {
        if (!cancelled) for (const node of ups) context.report({ node, message: cancel })
        for (const [node, e] of added) if (!removed.has(e)) context.report({ node, message: remove })
        if (!revoked) for (const node of made) context.report({ node, message: url })
        for (const [node, name, s] of engines) if (!freed.some(([n, f]) => n === name && f === s)) context.report({ node, message: engine })
      },
    }
  },
})
