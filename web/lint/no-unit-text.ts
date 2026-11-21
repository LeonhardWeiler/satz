import { defineRule, type ESTree } from 'vite-plus/lint/plugins'

const message = 'Show a length with length() from settings.ts, in the chosen unit.'
const UNIT = /\d\s+(?:mm|cm|pt|in)\b/

export default defineRule({
  create: (context) => {
    const check = (node: ESTree.Node, text: unknown) => typeof text === 'string' && UNIT.test(text) && context.report({ node, message })
    return {
      Literal: (node) => check(node, node.value),
      TemplateElement: (node) => check(node, node.value.raw),
      JSXText: (node) => check(node, node.value),
    }
  },
})
