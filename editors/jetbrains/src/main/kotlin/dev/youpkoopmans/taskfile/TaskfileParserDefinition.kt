package dev.youpkoopmans.taskfile

import com.intellij.lang.ASTNode
import com.intellij.lang.ParserDefinition
import com.intellij.lang.PsiParser
import com.intellij.lexer.Lexer
import com.intellij.openapi.project.Project
import com.intellij.psi.FileViewProvider
import com.intellij.psi.PsiElement
import com.intellij.psi.PsiFile
import com.intellij.psi.tree.IFileElementType
import com.intellij.psi.tree.TokenSet
import com.intellij.psi.impl.source.PsiPlainTextFileImpl
import com.intellij.extapi.psi.ASTWrapperPsiElement

class TaskfileParserDefinition : ParserDefinition {
    companion object {
        val FILE = IFileElementType(TaskfileLanguage.INSTANCE)
        val COMMENTS = TokenSet.create(TaskfileTokenTypes.COMMENT)
        val STRINGS = TokenSet.create(TaskfileTokenTypes.STRING)
    }

    override fun createLexer(project: Project?): Lexer = TaskfileLexer()

    override fun createParser(project: Project?): PsiParser = PsiParser { root, builder ->
        val mark = builder.mark()
        while (!builder.eof()) {
            builder.advanceLexer()
        }
        mark.done(root)
        builder.treeBuilt
    }

    override fun getFileNodeType(): IFileElementType = FILE

    override fun getCommentTokens(): TokenSet = COMMENTS

    override fun getStringLiteralElements(): TokenSet = STRINGS

    override fun createElement(node: ASTNode?): PsiElement = ASTWrapperPsiElement(node!!)

    override fun createFile(viewProvider: FileViewProvider): PsiFile = PsiPlainTextFileImpl(viewProvider)
}
