package dev.youpkoopmans.taskfile

import com.intellij.lang.Language

class TaskfileLanguage : Language("Taskfile") {
    companion object {
        @JvmStatic
        val INSTANCE = TaskfileLanguage()
    }
}
