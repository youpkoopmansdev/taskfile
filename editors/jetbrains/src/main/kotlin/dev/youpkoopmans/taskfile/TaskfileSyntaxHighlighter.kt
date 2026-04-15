package dev.youpkoopmans.taskfile

import com.intellij.lexer.Lexer
import com.intellij.openapi.editor.DefaultLanguageHighlighterColors
import com.intellij.openapi.editor.colors.TextAttributesKey
import com.intellij.openapi.fileTypes.SyntaxHighlighterBase
import com.intellij.psi.tree.IElementType

class TaskfileSyntaxHighlighter : SyntaxHighlighterBase() {

    companion object {
        val COMMENT = TextAttributesKey.createTextAttributesKey(
            "TASKFILE_COMMENT", DefaultLanguageHighlighterColors.LINE_COMMENT
        )
        val KEYWORD = TextAttributesKey.createTextAttributesKey(
            "TASKFILE_KEYWORD", DefaultLanguageHighlighterColors.KEYWORD
        )
        val ANNOTATION = TextAttributesKey.createTextAttributesKey(
            "TASKFILE_ANNOTATION", DefaultLanguageHighlighterColors.METADATA
        )
        val STRING = TextAttributesKey.createTextAttributesKey(
            "TASKFILE_STRING", DefaultLanguageHighlighterColors.STRING
        )
        val IDENTIFIER = TextAttributesKey.createTextAttributesKey(
            "TASKFILE_IDENTIFIER", DefaultLanguageHighlighterColors.IDENTIFIER
        )
        val NUMBER = TextAttributesKey.createTextAttributesKey(
            "TASKFILE_NUMBER", DefaultLanguageHighlighterColors.NUMBER
        )
        val BRACES = TextAttributesKey.createTextAttributesKey(
            "TASKFILE_BRACES", DefaultLanguageHighlighterColors.BRACES
        )
        val BRACKETS = TextAttributesKey.createTextAttributesKey(
            "TASKFILE_BRACKETS", DefaultLanguageHighlighterColors.BRACKETS
        )
        val EQUALS = TextAttributesKey.createTextAttributesKey(
            "TASKFILE_EQUALS", DefaultLanguageHighlighterColors.OPERATION_SIGN
        )
    }

    override fun getHighlightingLexer(): Lexer = TaskfileLexer()

    override fun getTokenHighlights(tokenType: IElementType): Array<TextAttributesKey> {
        return when (tokenType) {
            TaskfileTokenTypes.COMMENT -> arrayOf(COMMENT)
            TaskfileTokenTypes.KEYWORD -> arrayOf(KEYWORD)
            TaskfileTokenTypes.ANNOTATION -> arrayOf(ANNOTATION)
            TaskfileTokenTypes.STRING -> arrayOf(STRING)
            TaskfileTokenTypes.IDENTIFIER -> arrayOf(IDENTIFIER)
            TaskfileTokenTypes.NUMBER -> arrayOf(NUMBER)
            TaskfileTokenTypes.BRACE_OPEN, TaskfileTokenTypes.BRACE_CLOSE -> arrayOf(BRACES)
            TaskfileTokenTypes.BRACKET_OPEN, TaskfileTokenTypes.BRACKET_CLOSE -> arrayOf(BRACKETS)
            TaskfileTokenTypes.EQUALS -> arrayOf(EQUALS)
            else -> emptyArray()
        }
    }
}
