package dev.youpkoopmans.taskfile

import com.intellij.psi.tree.IElementType

class TaskfileTokenType(debugName: String) : IElementType(debugName, TaskfileLanguage.INSTANCE)

object TaskfileTokenTypes {
    @JvmField val COMMENT = TaskfileTokenType("COMMENT")
    @JvmField val KEYWORD = TaskfileTokenType("KEYWORD")
    @JvmField val ANNOTATION = TaskfileTokenType("ANNOTATION")
    @JvmField val STRING = TaskfileTokenType("STRING")
    @JvmField val IDENTIFIER = TaskfileTokenType("IDENTIFIER")
    @JvmField val NUMBER = TaskfileTokenType("NUMBER")
    @JvmField val BRACE_OPEN = TaskfileTokenType("BRACE_OPEN")
    @JvmField val BRACE_CLOSE = TaskfileTokenType("BRACE_CLOSE")
    @JvmField val BRACKET_OPEN = TaskfileTokenType("BRACKET_OPEN")
    @JvmField val BRACKET_CLOSE = TaskfileTokenType("BRACKET_CLOSE")
    @JvmField val EQUALS = TaskfileTokenType("EQUALS")
    @JvmField val WHITESPACE = TaskfileTokenType("WHITESPACE")
    @JvmField val BAD_CHARACTER = TaskfileTokenType("BAD_CHARACTER")
}
