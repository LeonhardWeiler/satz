import { defineRule, type ESTree } from 'vite-plus/lint/plugins'
import { called } from './match.ts'

const message = 'A cached promise keeps its rejection; cache it with once() from file.ts.'

const promised = (n: ESTree.Expression): boolean => {
  if (n.type === 'NewExpression') return called(n.callee, 'Promise')
  if (n.type === 'CallExpression' && called(n.callee, 'then')) return true
  for (let e: ESTree.Expression | ESTree.Super = n; e.type === 'CallExpression' || e.type === 'MemberExpression'; e = e.type === 'CallExpression' ? e.callee : e.object)
    if (e.type === 'CallExpression' && called(e.callee, 'fetch')) return true
  return false
}

export default defineRule({
  create: (context) => ({
    AssignmentExpression(node) {
      if (node.operator === '??=' && promised(node.right)) context.report({ node, message })
    },
  }),
})
