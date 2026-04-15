package dev.youpkoopmans.taskfile

import com.intellij.openapi.fileTypes.LanguageFileType
import javax.swing.Icon

class TaskfileFileType : LanguageFileType(TaskfileLanguage.INSTANCE) {
    companion object {
        @JvmStatic
        val INSTANCE = TaskfileFileType()
    }

    override fun getName(): String = "Taskfile"
    override fun getDescription(): String = "Taskfile task runner configuration"
    override fun getDefaultExtension(): String = "Taskfile"
    override fun getIcon(): Icon = TaskfileIcons.FILE
}
