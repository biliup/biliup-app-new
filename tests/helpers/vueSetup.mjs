import ts from 'typescript'

const setupAst = component => {
    const script = component.match(/<script setup lang="ts">([\s\S]*?)<\/script>/)[1]
    return ts.createSourceFile('Component.ts', script, ts.ScriptTarget.Latest, true)
}

export const compileSetup = component => {
    const ast = setupAst(component)
    const source = ast.statements
        .filter(statement => !ts.isImportDeclaration(statement))
        .map(statement => statement.getText(ast))
        .join('\n')
    return ts.transpile(source, { target: ts.ScriptTarget.ES2020 })
}

export const setupMethod = (component, name) => {
    const ast = setupAst(component)
    const statement = ast.statements.find(statement =>
        ts.isVariableStatement(statement) && statement.declarationList.declarations.some(
            declaration => declaration.name.getText(ast) === name
        )
    )
    return ts.transpile(statement.getText(ast), { target: ts.ScriptTarget.ES2020 })
}
