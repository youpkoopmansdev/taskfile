package dev.youpkoopmans.taskfile

import com.intellij.execution.configurations.GeneralCommandLine
import com.intellij.openapi.project.Project
import com.intellij.openapi.vfs.VirtualFile
import com.intellij.platform.lsp.api.LspServerSupportProvider
import com.intellij.platform.lsp.api.ProjectWideLspServerDescriptor

class TaskfileLspServerSupportProvider : LspServerSupportProvider {
    override fun fileOpened(
        project: Project,
        file: VirtualFile,
        serverStarter: LspServerSupportProvider.LspServerStarter
    ) {
        if (file.fileType == TaskfileFileType.INSTANCE || file.name == "Taskfile" || file.name.endsWith(".Taskfile")) {
            serverStarter.ensureServerStarted(TaskfileLspServerDescriptor(project))
        }
    }
}

class TaskfileLspServerDescriptor(project: Project) : ProjectWideLspServerDescriptor(project, "Taskfile") {
    override fun isSupportedFile(file: VirtualFile): Boolean {
        return file.fileType == TaskfileFileType.INSTANCE || file.name == "Taskfile" || file.name.endsWith(".Taskfile")
    }

    override fun createCommandLine(): GeneralCommandLine {
        return GeneralCommandLine("taskfile-lsp")
    }
}
