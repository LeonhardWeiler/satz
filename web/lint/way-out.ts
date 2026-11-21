import { defineRule, type ESTree } from 'vite-plus/lint/plugins'
import { called } from './match.ts'

const message = 'A file that calls URL.createObjectURL also calls URL.revokeObjectURL.'

export default defineRule({
  create: (context) => {
    const made: ESTree.Node[] = []
    let revoked = false
    return {
      CallExpression(node) {
        if (called(node.callee, 'createObjectURL', 'URL')) made.push(node)
        if (called(node.callee, 'revokeObjectURL', 'URL')) revoked = true
      },
      'Program:exit'() {
        if (!revoked) for (const node of made) context.report({ node, message })
      },
    }
  },
})
