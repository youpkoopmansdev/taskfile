package dev.youpkoopmans.taskfile

import com.intellij.lexer.LexerBase
import com.intellij.psi.tree.IElementType

class TaskfileLexer : LexerBase() {
    private var buffer: CharSequence = ""
    private var bufferEnd: Int = 0
    private var tokenStart: Int = 0
    private var tokenEnd: Int = 0
    private var currentToken: IElementType? = null

    companion object {
        private val KEYWORDS = setOf(
            "task", "export", "alias", "include", "dotenv",
            "depends", "depends_parallel"
        )
    }

    override fun start(buffer: CharSequence, startOffset: Int, endOffset: Int, initialState: Int) {
        this.buffer = buffer
        this.bufferEnd = endOffset
        this.tokenStart = startOffset
        this.tokenEnd = startOffset
        advance()
    }

    override fun getState(): Int = 0
    override fun getTokenType(): IElementType? = currentToken
    override fun getTokenStart(): Int = tokenStart
    override fun getTokenEnd(): Int = tokenEnd
    override fun getBufferSequence(): CharSequence = buffer
    override fun getBufferEnd(): Int = bufferEnd

    override fun advance() {
        tokenStart = tokenEnd
        if (tokenStart >= bufferEnd) {
            currentToken = null
            return
        }

        val c = buffer[tokenStart]

        // Whitespace
        if (c == ' ' || c == '\t' || c == '\n' || c == '\r') {
            tokenEnd = tokenStart + 1
            while (tokenEnd < bufferEnd) {
                val w = buffer[tokenEnd]
                if (w != ' ' && w != '\t' && w != '\n' && w != '\r') break
                tokenEnd++
            }
            currentToken = TaskfileTokenTypes.WHITESPACE
            return
        }

        // Comment
        if (c == '#') {
            tokenEnd = tokenStart + 1
            while (tokenEnd < bufferEnd && buffer[tokenEnd] != '\n') {
                tokenEnd++
            }
            currentToken = TaskfileTokenTypes.COMMENT
            return
        }

        // String
        if (c == '"') {
            tokenEnd = tokenStart + 1
            while (tokenEnd < bufferEnd) {
                val sc = buffer[tokenEnd]
                if (sc == '\\' && tokenEnd + 1 < bufferEnd) {
                    tokenEnd += 2
                } else if (sc == '"') {
                    tokenEnd++
                    break
                } else if (sc == '\n') {
                    break
                } else {
                    tokenEnd++
                }
            }
            currentToken = TaskfileTokenTypes.STRING
            return
        }

        // Annotation (@description, @confirm)
        if (c == '@') {
            if (lookingAt("@description") && !isIdentChar(tokenStart + 12)) {
                tokenEnd = tokenStart + 12
                currentToken = TaskfileTokenTypes.ANNOTATION
                return
            }
            if (lookingAt("@confirm") && !isIdentChar(tokenStart + 8)) {
                tokenEnd = tokenStart + 8
                currentToken = TaskfileTokenTypes.ANNOTATION
                return
            }
            tokenEnd = tokenStart + 1
            currentToken = TaskfileTokenTypes.BAD_CHARACTER
            return
        }

        // Brackets and braces
        when (c) {
            '[' -> { tokenEnd = tokenStart + 1; currentToken = TaskfileTokenTypes.BRACKET_OPEN; return }
            ']' -> { tokenEnd = tokenStart + 1; currentToken = TaskfileTokenTypes.BRACKET_CLOSE; return }
            '{' -> { tokenEnd = tokenStart + 1; currentToken = TaskfileTokenTypes.BRACE_OPEN; return }
            '}' -> { tokenEnd = tokenStart + 1; currentToken = TaskfileTokenTypes.BRACE_CLOSE; return }
            '=' -> { tokenEnd = tokenStart + 1; currentToken = TaskfileTokenTypes.EQUALS; return }
        }

        // Identifiers and keywords
        if (c.isLetter() || c == '_') {
            tokenEnd = tokenStart + 1
            while (tokenEnd < bufferEnd && isIdentChar(tokenEnd)) {
                tokenEnd++
            }
            val word = buffer.subSequence(tokenStart, tokenEnd).toString()
            currentToken = if (word in KEYWORDS) TaskfileTokenTypes.KEYWORD else TaskfileTokenTypes.IDENTIFIER
            return
        }

        // Numbers
        if (c.isDigit()) {
            tokenEnd = tokenStart + 1
            while (tokenEnd < bufferEnd && buffer[tokenEnd].isDigit()) {
                tokenEnd++
            }
            currentToken = TaskfileTokenTypes.NUMBER
            return
        }

        // Everything else
        tokenEnd = tokenStart + 1
        currentToken = TaskfileTokenTypes.BAD_CHARACTER
    }

    private fun lookingAt(prefix: String): Boolean {
        if (tokenStart + prefix.length > bufferEnd) return false
        for (i in prefix.indices) {
            if (buffer[tokenStart + i] != prefix[i]) return false
        }
        return true
    }

    private fun isIdentChar(pos: Int): Boolean {
        if (pos >= bufferEnd) return false
        val ch = buffer[pos]
        return ch.isLetterOrDigit() || ch == '_' || ch == '-'
    }
}
