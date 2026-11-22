import { defineRule, type ESTree } from 'vite-plus/lint/plugins'

const message = 'Name the action with data-action; the tooltip reads its keys.'
const SHORTCUT = /\((?:(?:Ctrl|Alt|Shift)[ +])*(?:[A-Z0-9]|[^\w\s)]|PgUp|PgDn|Home|End|Del|Tab|F\d+)\)/

export default defineRule({
  create: (context) => {
    const check = (node: ESTree.Node, text: unknown) => typeof text === 'string' && SHORTCUT.test(text) && context.report({ node, message })
    const inTitle = (node: ESTree.Node) => {
      for (let n = node.parent; n; n = n.parent) if (n.type === 'JSXAttribute') return n.name.type === 'JSXIdentifier' && n.name.name === 'title'
      return false
    }
    return {
      Literal: (node) => inTitle(node) && check(node, node.value),
      TemplateElement: (node) => inTitle(node) && check(node, node.value.raw),
    }
  },
})
