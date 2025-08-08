<template>
    <div class="main-view">
        <!-- 拖拽覆盖层 -->
        <div v-if="isDragOver" class="drag-overlay">
            <div class="drag-content">
                <el-icon class="drag-icon"><upload-filled /></el-icon>
                <h3>拖拽视频文件到此处</h3>
                <p>支持 MP4、AVI、MOV、MKV、WMV、FLV、M4V、WEBM 格式</p>
                <p v-if="!selectedUser || !currentTemplateName" class="warning-text">
                    请先选择用户和模板
                </p>
            </div>
        </div>

        <!-- 顶部导航栏 -->
        <el-header class="header">
            <div class="header-content">
                <div class="header-left">
                    <h2 class="app-title">Biliup APP</h2>
                </div>
                <div class="header-right">
                    <!-- 上传队列下拉框 -->
                    <UploadQueue />

                    <!-- 全局设置按钮 -->
                    <el-button
                        type="info"
                        size="small"
                        circle
                        @click="showGlobalConfigDialog = true"
                        title="全局设置"
                        class="global-config-btn"
                    >
                        <el-icon><setting /></el-icon>
                    </el-button>

                    <!-- 用户信息显示 - 显示所有登录用户数量 -->
                    <div class="user-info">
                        <el-icon><user /></el-icon>
                        <span>{{ loginUsers.length }} 个用户已登录</span>
                    </div>
                </div>
            </div>
        </el-header>

        <el-container class="main-container">
            <!-- 用户模板侧边栏 -->
            <el-aside width="300px" class="sidebar">
                <div class="sidebar-header">
                    <h3></h3>
                    <div class="header-buttons">
                        <el-button type="success" size="small" @click="showLoginDialog = true">
                            <el-icon><user /></el-icon>
                            登录用户
                        </el-button>
                        <el-button
                            type="primary"
                            size="small"
                            @click="showCreateTemplateDialog = true"
                            :disabled="!loginUsers.length"
                        >
                            <el-icon><plus /></el-icon>
                            新建模板
                        </el-button>
                    </div>
                </div>

                <div class="sidebar-content">
                    <div class="user-template-list">
                        <div
                            v-for="userTemplate in userTemplates"
                            :key="userTemplate.user.uid"
                            class="user-section"
                        >
                            <!-- 用户头部 -->
                            <div
                                class="user-header"
                                @click="toggleUserExpanded(userTemplate.user.uid)"
                            >
                                <el-avatar
                                    :src="`data:image/jpeg;base64,${userTemplate.user.avatar}`"
                                    :size="24"
                                    class="user-avatar"
                                >
                                    {{ userTemplate.user.username.charAt(0) }}
                                </el-avatar>
                                <span class="user-name">{{ userTemplate.user.username }}</span>
                                <el-tooltip
                                    :content="
                                        isUserHasUploadTasks(userTemplate.user.uid)
                                            ? '请先删除上传队列中属于该用户的任务'
                                            : '登出用户'
                                    "
                                    placement="top"
                                >
                                    <el-icon
                                        class="logout-icon"
                                        :class="{
                                            disabled: isUserHasUploadTasks(userTemplate.user.uid)
                                        }"
                                        @click.stop="handleLogoutUser(userTemplate.user.uid)"
                                        title="登出用户"
                                    >
                                        <close />
                                    </el-icon>
                                </el-tooltip>
                                <el-icon
                                    class="config-icon"
                                    @click.stop="openUserConfig(userTemplate.user)"
                                    title="用户配置"
                                >
                                    <setting />
                                </el-icon>
                                <el-badge
                                    :value="userTemplate.templates.length"
                                    class="template-count-badge"
                                />
                                <el-icon
                                    class="expand-icon"
                                    :class="{ expanded: userTemplate.expanded }"
                                >
                                    <arrow-down />
                                </el-icon>
                            </div>

                            <!-- 模板列表 -->
                            <div class="template-list" v-show="userTemplate.expanded">
                                <div
                                    v-for="template in userTemplate.templates"
                                    :key="`${userTemplate.user.uid}-${template.name}`"
                                    class="template-item"
                                    :class="{
                                        active:
                                            selectedUser?.uid === userTemplate.user.uid &&
                                            currentTemplateName === template.name
                                    }"
                                    @click="selectTemplate(userTemplate.user, template.name)"
                                >
                                    <div class="template-main">
                                        <div class="template-name">
                                            {{ template.name }}
                                            <span
                                                v-if="
                                                    selectedUser?.uid === userTemplate.user.uid &&
                                                    currentTemplateName === template.name &&
                                                    hasUnsavedChanges
                                                "
                                                class="unsaved-indicator"
                                                title="有未保存的修改"
                                            ></span>
                                        </div>
                                        <div class="template-desc">
                                            {{ template.config.title || '无标题' }}
                                        </div>
                                    </div>
                                    <el-dropdown
                                        @command="
                                            (command: string) =>
                                                handleTemplateCommand(
                                                    command,
                                                    userTemplate.user,
                                                    template
                                                )
                                        "
                                        @click.stop
                                        trigger="click"
                                    >
                                        <el-button link size="small" class="template-menu-btn">
                                            <el-icon><more-filled /></el-icon>
                                        </el-button>
                                        <template #dropdown>
                                            <el-dropdown-menu>
                                                <el-dropdown-item command="duplicate"
                                                    >复制</el-dropdown-item
                                                >
                                                <el-dropdown-item command="rename"
                                                    >重命名</el-dropdown-item
                                                >
                                                <el-dropdown-item command="delete" divided
                                                    >删除</el-dropdown-item
                                                >
                                            </el-dropdown-menu>
                                        </template>
                                    </el-dropdown>
                                </div>
                            </div>
                        </div>

                        <!-- 空状态 -->
                        <div v-if="userTemplates.length === 0" class="empty-users">
                            <el-empty description="暂无登录用户">
                                <el-button type="primary" @click="showLoginDialog = true">
                                    去登录
                                </el-button>
                            </el-empty>
                        </div>
                    </div>
                </div>
            </el-aside>

            <!-- 主要内容区域 -->
            <el-main class="main-content">
                <div class="content-wrapper" ref="contentWrapperRef">
                    <div v-if="!selectedUser" class="no-selection">
                        <el-empty description="请选择用户和模板开始使用" />
                    </div>

                    <div v-else-if="!currentTemplateName" class="no-template">
                        <el-empty description="请选择模板或创建新模板">
                            <el-button type="primary" @click="showCreateTemplateDialog = true">
                                新建模板
                            </el-button>
                        </el-empty>
                    </div>

                    <div v-else class="upload-form-container">
                        <div class="form-header">
                            <div class="template-name-container">
                                <h3
                                    v-if="!isEditingTemplateName"
                                    @click="startEditTemplateName"
                                    class="template-name-display"
                                    :title="'点击编辑模板名称'"
                                >
                                    {{ currentTemplateName }}
                                    <el-icon class="edit-hint-icon"><edit /></el-icon>
                                </h3>
                                <el-input
                                    v-else
                                    ref="templateNameInputRef"
                                    v-model="editingTemplateName"
                                    @blur="saveTemplateName"
                                    @keyup.enter="saveTemplateName"
                                    @keyup.esc="cancelEditTemplateName"
                                    class="template-name-input"
                                    size="large"
                                />
                            </div>
                            <div class="header-actions">
                                <el-button @click="loadTemplateToForm">重置</el-button>
                                <el-button type="primary" @click="saveTemplate">保存</el-button>
                                <el-button
                                    @click="
                                        handleTemplateCommand('delete', selectedUser, {
                                            name: currentTemplateName,
                                            config: currentTemplate
                                        })
                                    "
                                    @click.stop
                                    trigger="click"
                                    type="danger"
                                    >删除</el-button
                                >
                            </div>
                        </div>

                        <el-form :model="currentForm" label-width="120px" class="upload-form">
                            <!-- 基本信息 -->
                            <el-card
                                class="form-section"
                                :class="{ collapsed: cardCollapsed.basic }"
                            >
                                <template #header>
                                    <div class="card-header" @click="toggleCardCollapsed('basic')">
                                        <span>基本信息</span>
                                        <el-icon
                                            class="collapse-icon"
                                            :class="{ collapsed: cardCollapsed.basic }"
                                        >
                                            <arrow-down />
                                        </el-icon>
                                    </div>
                                </template>

                                <el-collapse-transition>
                                    <div v-show="!cardCollapsed.basic" class="card-content">
                                        <el-form-item label="视频标题" required>
                                            <el-input
                                                v-model="currentForm.title"
                                                placeholder="请输入视频标题"
                                                maxlength="80"
                                                show-word-limit
                                            />
                                        </el-form-item>

                                        <el-form-item label="封面">
                                            <div
                                                class="cover-uploader"
                                                action="#"
                                                @click="selectCoverWithTauri"
                                            >
                                                <img
                                                    v-if="coverDisplayUrl"
                                                    :src="coverDisplayUrl"
                                                    class="cover-image"
                                                />
                                                <el-icon v-else class="cover-uploader-icon"
                                                    ><plus
                                                /></el-icon>
                                            </div>
                                        </el-form-item>

                                        <el-form-item label="视频分区">
                                            <el-popover
                                                v-model:visible="categoryPopoverVisible"
                                                placement="bottom-start"
                                                :width="600"
                                                trigger="click"
                                                popper-class="category-popover"
                                            >
                                                <template #reference>
                                                    <el-button
                                                        class="category-trigger"
                                                        :type="
                                                            currentForm.tid ? 'primary' : 'default'
                                                        "
                                                    >
                                                        <span class="category-text">
                                                            <span v-if="selectedSubCategory">
                                                                {{ selectedCategory?.name }} >
                                                                {{ selectedSubCategory?.name }}
                                                            </span>
                                                            <span v-else class="placeholder"
                                                                >请选择分区</span
                                                            >
                                                        </span>
                                                        <el-icon class="arrow-icon">
                                                            <arrow-down />
                                                        </el-icon>
                                                    </el-button>
                                                </template>

                                                <div class="category-selector-panel">
                                                    <!-- 左侧主分区列表 -->
                                                    <div class="category-list">
                                                        <div
                                                            v-for="category in typeList"
                                                            :key="category.id"
                                                            class="category-item"
                                                            :class="{
                                                                active:
                                                                    selectedCategory?.id ===
                                                                    category.id
                                                            }"
                                                            @click="onCategoryChange(category.id)"
                                                        >
                                                            <span class="category-name">{{
                                                                category.name
                                                            }}</span>
                                                            <el-icon class="arrow-right">
                                                                <arrow-down
                                                                    style="
                                                                        transform: rotate(-90deg);
                                                                    "
                                                                />
                                                            </el-icon>
                                                        </div>
                                                    </div>

                                                    <!-- 右侧子分区列表 -->
                                                    <div class="subcategory-list">
                                                        <div
                                                            v-if="
                                                                selectedCategory &&
                                                                selectedCategory.children
                                                            "
                                                        >
                                                            <div
                                                                v-for="subCategory in selectedCategory.children"
                                                                :key="subCategory.id"
                                                                class="subcategory-item"
                                                                :class="{
                                                                    active:
                                                                        selectedSubCategory?.id ===
                                                                        subCategory.id
                                                                }"
                                                                @click="
                                                                    onSubCategoryChange(
                                                                        subCategory.id
                                                                    )
                                                                "
                                                                :title="
                                                                    subCategory.intro_original ||
                                                                    subCategory.desc
                                                                "
                                                            >
                                                                <div class="subcategory-content">
                                                                    <div class="subcategory-name">
                                                                        {{ subCategory.name }}
                                                                    </div>
                                                                    <div class="subcategory-desc">
                                                                        {{
                                                                            subCategory.desc !== ''
                                                                                ? subCategory.desc
                                                                                : subCategory.intro_original
                                                                        }}
                                                                    </div>
                                                                </div>
                                                            </div>
                                                        </div>
                                                        <div v-else class="empty-subcategory">
                                                            <el-empty
                                                                description="请选择左侧主分区"
                                                                :image-size="60"
                                                            />
                                                        </div>
                                                    </div>
                                                </div>
                                            </el-popover>
                                        </el-form-item>

                                        <el-form-item label="版权声明">
                                            <el-radio-group v-model="currentForm.copyright">
                                                <el-radio :value="1">自制</el-radio>
                                                <el-radio :value="2">转载</el-radio>
                                            </el-radio-group>
                                        </el-form-item>

                                        <el-form-item
                                            label="转载来源"
                                            v-if="currentForm.copyright === 2"
                                        >
                                            <el-input
                                                v-model="currentForm.source"
                                                placeholder="请填写转载来源"
                                            />
                                        </el-form-item>
                                    </div>
                                </el-collapse-transition>
                            </el-card>

                            <!-- 标签设置 -->
                            <el-card
                                class="form-section"
                                :class="{ collapsed: cardCollapsed.tags }"
                            >
                                <template #header>
                                    <div class="card-header" @click="toggleCardCollapsed('tags')">
                                        <span>标签设置</span>
                                        <el-icon
                                            class="collapse-icon"
                                            :class="{ collapsed: cardCollapsed.tags }"
                                        >
                                            <arrow-down />
                                        </el-icon>
                                    </div>
                                </template>

                                <el-collapse-transition>
                                    <div v-show="!cardCollapsed.tags" class="card-content">
                                        <el-form-item label="视频标签">
                                            <div class="tag-input">
                                                <el-tag
                                                    v-for="tag in tags"
                                                    :key="tag"
                                                    closable
                                                    @close="removeTag(tag)"
                                                    class="tag-item"
                                                >
                                                    {{ tag }}
                                                </el-tag>
                                                <el-input
                                                    v-if="inputVisible"
                                                    ref="tagInputRef"
                                                    v-model="newTag"
                                                    size="small"
                                                    placeholder="按回车键添加"
                                                    @keyup.enter="addTag"
                                                    @blur="addTag"
                                                    class="tag-input-field"
                                                />
                                                <el-button
                                                    v-else
                                                    size="small"
                                                    @click="showTagInput"
                                                    class="add-tag-btn"
                                                >
                                                    + 添加标签
                                                </el-button>
                                            </div>
                                        </el-form-item>
                                    </div>
                                </el-collapse-transition>
                            </el-card>

                            <!-- 视频描述 -->
                            <el-card
                                class="form-section"
                                :class="{ collapsed: cardCollapsed.description }"
                            >
                                <template #header>
                                    <div
                                        class="card-header"
                                        @click="toggleCardCollapsed('description')"
                                    >
                                        <span>视频描述</span>
                                        <el-icon
                                            class="collapse-icon"
                                            :class="{ collapsed: cardCollapsed.description }"
                                        >
                                            <arrow-down />
                                        </el-icon>
                                    </div>
                                </template>

                                <el-collapse-transition>
                                    <div v-show="!cardCollapsed.description" class="card-content">
                                        <el-form-item label="简介">
                                            <el-input
                                                v-model="currentForm.desc"
                                                type="textarea"
                                                :rows="6"
                                                placeholder="请输入视频简介"
                                                maxlength="2000"
                                                show-word-limit
                                            />
                                        </el-form-item>

                                        <el-form-item label="粉丝动态">
                                            <el-input
                                                v-model="currentForm.dynamic"
                                                placeholder="发布时的动态内容"
                                                maxlength="233"
                                                show-word-limit
                                            />
                                        </el-form-item>
                                    </div>
                                </el-collapse-transition>
                            </el-card>

                            <!-- 视频文件 -->
                            <el-card
                                class="form-section"
                                :class="{
                                    'drag-target': isDragOver,
                                    collapsed: cardCollapsed.videos
                                }"
                            >
                                <template #header>
                                    <div class="card-header" @click="toggleCardCollapsed('videos')">
                                        <span>视频文件</span>
                                        <span v-if="isDragOver" class="drag-hint"
                                            >拖拽文件到此处添加</span
                                        >
                                        <el-icon
                                            class="collapse-icon"
                                            :class="{ collapsed: cardCollapsed.videos }"
                                        >
                                            <arrow-down />
                                        </el-icon>
                                    </div>
                                </template>

                                <el-collapse-transition>
                                    <div v-show="!cardCollapsed.videos" class="card-content">
                                        <!-- 已上传文件列表 -->
                                        <div
                                            v-if="currentForm.videos && currentForm.videos.length > 0"
                                            class="uploaded-videos-section"
                                        >
                                            <div class="uploaded-videos-list">
                                                <div
                                                    v-for="video in updateVideo(currentForm.videos)"
                                                    :key="video.id"
                                                    class="uploaded-video-item"
                                                >
                                                    <div class="video-status-icon">
                                                        <!-- 上传完成 -->
                                                        <el-icon
                                                            v-if="video.complete"
                                                            class="status-complete"
                                                        >
                                                            <circle-check />
                                                        </el-icon>
                                                        <!-- 上传中 -->
                                                        <el-icon
                                                            v-else-if="
                                                                !video.complete &&
                                                                video.progress > 0
                                                            "
                                                            class="status-uploading"
                                                        >
                                                            <loading />
                                                        </el-icon>
                                                        <!-- 待上传 -->
                                                        <el-icon v-else class="status-pending">
                                                            <cloudy />
                                                        </el-icon>
                                                    </div>
                                                    <div class="video-info">
                                                        <!-- 文件名和状态在同一行 -->
                                                        <div class="video-title-row">
                                                            <div class="video-title-container">
                                                                <div
                                                                    v-if="
                                                                        editingFileId === video.id
                                                                    "
                                                                    class="video-title-edit"
                                                                >
                                                                    <el-input
                                                                        v-model="editingFileName"
                                                                        size="small"
                                                                        @keyup.enter="
                                                                            saveFileName(video.id)
                                                                        "
                                                                        @blur="
                                                                            saveFileName(video.id)
                                                                        "
                                                                        @keyup.esc="
                                                                            cancelEditFileName
                                                                        "
                                                                        ref="videoNameInput"
                                                                    />
                                                                </div>
                                                                <div
                                                                    v-else
                                                                    class="video-title"
                                                                    @click="
                                                                        startEditFileName(
                                                                            video.id,
                                                                            video.title ||
                                                                                video.videoname
                                                                        )
                                                                    "
                                                                >
                                                                    {{
                                                                        video.title ||
                                                                        video.videoname
                                                                    }}
                                                                    <el-icon class="edit-icon"
                                                                        ><edit
                                                                    /></el-icon>
                                                                </div>
                                                            </div>

                                                            <!-- 状态标签移动到文件名右侧 -->
                                                            <div class="video-status">
                                                                <span
                                                                    v-if="video.complete"
                                                                    class="status-text complete"
                                                                    >上传完成</span
                                                                >
                                                                <span
                                                                    v-else-if="video.progress > 0"
                                                                    class="status-text uploading"
                                                                >
                                                                    上传中
                                                                </span>
                                                                <span
                                                                    v-else
                                                                    class="status-text pending"
                                                                    >待上传</span
                                                                >
                                                            </div>
                                                        </div>

                                                        <!-- 进度条区域 -->
                                                        <div class="progress-section">
                                                            <div class="progress-bar-container">
                                                                <el-progress
                                                                    :percentage="video.progress"
                                                                    :show-text="false"
                                                                    size="small"
                                                                    :stroke-width="3"
                                                                    :color="
                                                                        video.complete
                                                                            ? '#67c23a'
                                                                            : '#409eff'
                                                                    "
                                                                />
                                                                <span class="progress-text"
                                                                    >{{
                                                                        formatUploadProgress(video)
                                                                    }}%</span
                                                                >
                                                            </div>
                                                            <div
                                                                class="upload-speed"
                                                                v-if="
                                                                    !video.complete &&
                                                                    video.speed > 0
                                                                "
                                                            >
                                                                {{ formatUploadSpeed(video) }}
                                                            </div>
                                                        </div>
                                                    </div>

                                                    <!-- 文件操作按钮 -->
                                                    <div class="video-actions">
                                                        <el-button
                                                            type="danger"
                                                            size="small"
                                                            text
                                                            @click="removeUploadedFile(video.id)"
                                                        >
                                                            <el-icon><delete /></el-icon>
                                                        </el-button>
                                                    </div>
                                                </div>
                                            </div>
                                        </div>

                                        <el-form-item label="">
                                            <div class="video-buttons-group">
                                                <el-button
                                                    type="primary"
                                                    @click="selectVideoWithTauri"
                                                >
                                                    <el-icon><upload-filled /></el-icon>
                                                    选择视频文件
                                                </el-button>
                                                <el-button
                                                    type="danger"
                                                    plain
                                                    @click="clearAllVideos"
                                                    :disabled="
                                                        !currentForm.videos ||
                                                        currentForm.videos.length === 0
                                                    "
                                                >
                                                    <el-icon><delete /></el-icon>
                                                    清空{{
                                                        currentForm.videos &&
                                                        currentForm.videos.length > 0
                                                            ? `(${currentForm.videos.length})`
                                                            : ''
                                                    }}
                                                </el-button>
                                            </div>
                                            <div class="upload-tip">
                                                <span v-if="!isDragOver">
                                                    支持 MP4、AVI、MOV、MKV、WMV、FLV、M4V、WEBM
                                                    等格式
                                                </span>
                                                <span v-else class="drag-active-tip">
                                                    💡 松开鼠标即可添加文件到当前模板
                                                </span>
                                            </div>
                                        </el-form-item>
                                    </div>
                                </el-collapse-transition>
                            </el-card>

                            <!-- 高级选项 -->
                            <el-card
                                class="form-section"
                                :class="{ collapsed: cardCollapsed.advanced }"
                            >
                                <template #header>
                                    <div
                                        class="card-header"
                                        @click="toggleCardCollapsed('advanced')"
                                    >
                                        <span>高级选项</span>
                                        <el-icon
                                            class="collapse-icon"
                                            :class="{ collapsed: cardCollapsed.advanced }"
                                        >
                                            <arrow-down />
                                        </el-icon>
                                    </div>
                                </template>

                                <el-collapse-transition>
                                    <div v-show="!cardCollapsed.advanced" class="card-content">
                                        <el-form-item label="定时发布">
                                            <el-date-picker
                                                v-model="dtimeDate"
                                                type="datetime"
                                                placeholder="选择发布时间"
                                                format="YYYY-MM-DD HH:mm:ss"
                                                :disabled-date="(date: Date) => date < new Date()"
                                            />
                                        </el-form-item>

                                        <el-form-item label="字幕设置">
                                            <el-checkbox v-model="currentForm.open_subtitle">
                                                开启字幕功能
                                            </el-checkbox>
                                        </el-form-item>

                                        <el-form-item label="活动/话题">
                                            <TopicView
                                                v-model="currentForm.mission_id"
                                                v-model:topic-id="currentForm.topic_id"
                                                :user-uid="selectedUser?.uid"
                                                mode="selector"
                                            />
                                        </el-form-item>

                                        <el-form-item label="互动功能">
                                            <el-switch
                                                v-model="currentForm.interactive"
                                                active-text="开启"
                                                inactive-text="关闭"
                                                :active-value="1"
                                                :inactive-value="0"
                                            />
                                        </el-form-item>

                                        <el-form-item
                                            label="加入合集"
                                            v-if="
                                                currentTemplate &&
                                                currentTemplate.aid &&
                                                currentTemplate.aid !== 0
                                            "
                                        >
                                            <SeasonView
                                                v-model="currentForm.season_id"
                                                v-model:section-id="currentForm.section_id"
                                                :user-uid="selectedUser?.uid"
                                                :aid="currentTemplate.aid"
                                            />
                                        </el-form-item>

                                        <el-form-item label="音质设置">
                                            <div class="checkbox-group">
                                                <el-checkbox
                                                    v-model="currentForm.dolby"
                                                    :true-value="1"
                                                    :false-value="0"
                                                >
                                                    杜比音效
                                                </el-checkbox>
                                                <el-checkbox
                                                    v-model="currentForm.lossless_music"
                                                    :true-value="1"
                                                    :false-value="0"
                                                >
                                                    无损音乐
                                                </el-checkbox>
                                            </div>
                                        </el-form-item>

                                        <el-form-item label="内容设置">
                                            <div class="checkbox-group">
                                                <el-checkbox
                                                    v-model="currentForm.no_reprint"
                                                    :true-value="1"
                                                    :false-value="0"
                                                >
                                                    禁止转载
                                                </el-checkbox>
                                                <el-checkbox
                                                    v-model="currentForm.open_elec"
                                                    :true-value="1"
                                                    :false-value="0"
                                                >
                                                    开启充电
                                                </el-checkbox>
                                            </div>
                                        </el-form-item>

                                        <el-form-item label="互动管理">
                                            <div class="checkbox-group">
                                                <el-checkbox
                                                    v-model="currentForm.up_selection_reply"
                                                    :true-value="1"
                                                    :false-value="0"
                                                >
                                                    UP主精选评论
                                                </el-checkbox>
                                                <el-checkbox
                                                    v-model="currentForm.up_close_reply"
                                                    :true-value="1"
                                                    :false-value="0"
                                                >
                                                    关闭评论
                                                </el-checkbox>
                                                <el-checkbox
                                                    v-model="currentForm.up_close_danmu"
                                                    :true-value="1"
                                                    :false-value="0"
                                                >
                                                    关闭弹幕
                                                </el-checkbox>
                                            </div>
                                        </el-form-item>

                                        <el-form-item label="可见性设置">
                                            <el-checkbox
                                                v-model="currentForm.is_only_self"
                                                :true-value="1"
                                                :false-value="0"
                                            >
                                                仅自己可见
                                            </el-checkbox>
                                        </el-form-item>
                                    </div>
                                </el-collapse-transition>
                            </el-card>

                            <!-- 上传操作区域 -->
                            <div class="upload-actions">
                                <el-button
                                    type="success"
                                    size="large"
                                    :loading="uploading"
                                    @click="createUpload"
                                >
                                    <el-icon><upload-filled /></el-icon>
                                    加入上传队列
                                </el-button>

                                <el-button
                                    type="primary"
                                    size="large"
                                    :loading="submitting"
                                    @click="submitTemplate(false)"
                                    :disabled="!currentForm.videos || currentForm.videos.length === 0"
                                >
                                    <el-icon v-if="!allFilesUploaded && !submitting"
                                        ><loading
                                    /></el-icon>
                                    <el-icon v-else-if="!submitting"><check /></el-icon>
                                    {{
                                        !autoSubmitting
                                            ? currentTemplate?.aid
                                                ? '编辑稿件'
                                                : '新增稿件'
                                            : '上传完成后自动提交'
                                    }}
                                </el-button>
                                <div class="form-tip" v-if="lastSubmit">
                                    最后提交时间: {{ lastSubmit }}
                                </div>
                            </div>
                        </el-form>
                    </div>
                </div>
            </el-main>
        </el-container>

        <!-- 创建模板对话框 -->
        <el-dialog v-model="showCreateTemplateDialog" title="新建模板" width="500px">
            <el-form :model="newTemplateForm" label-width="100px">
                <el-form-item label="选择用户">
                    <el-select v-model="newTemplateForm.userUid" placeholder="请选择用户">
                        <el-option
                            v-for="user in loginUsers"
                            :key="user.uid"
                            :label="user.username"
                            :value="user.uid"
                        >
                            <div class="user-option">
                                <el-avatar
                                    :src="`data:image/jpeg;base64,${user.avatar}`"
                                    :size="20"
                                >
                                    {{ user.username.charAt(0) }}
                                </el-avatar>
                                <span class="user-option-name">{{ user.username }}</span>
                            </div>
                        </el-option>
                    </el-select>
                </el-form-item>

                <el-form-item label="模板类型">
                    <el-radio-group v-model="newTemplateForm.templateType">
                        <el-radio value="blank">空白模板</el-radio>
                        <el-radio value="bv">BV/AV号</el-radio>
                    </el-radio-group>
                </el-form-item>

                <el-form-item
                    label="模板名称"
                    required
                    v-if="newTemplateForm.templateType === 'blank'"
                >
                    <el-input
                        v-model="newTemplateForm.name"
                        placeholder="请输入模板名称"
                        maxlength="50"
                    />
                </el-form-item>

                <el-form-item label="BV/AV号" required v-if="newTemplateForm.templateType === 'bv'">
                    <el-input
                        v-model="newTemplateForm.bvNumber"
                        placeholder="请输入BV号或AV号，如: BV1xx4y1z7xx 或 av12345"
                        maxlength="20"
                    />
                </el-form-item>

                <el-form-item label="操作类型" v-if="newTemplateForm.templateType === 'bv'">
                    <el-radio-group v-model="newTemplateForm.actionType">
                        <el-radio value="edit">编辑</el-radio>
                        <el-radio value="copy">复制</el-radio>
                    </el-radio-group>
                    <div class="form-tip">
                        <div>编辑：直接修改现有稿件</div>
                        <div>复制：基于现有稿件创建新模板</div>
                    </div>
                </el-form-item>
                <el-form-item
                    label="模板名称"
                    required
                    v-if="
                        newTemplateForm.templateType === 'bv' &&
                        newTemplateForm.actionType === 'copy'
                    "
                >
                    <el-input
                        v-model="newTemplateForm.name"
                        placeholder="请输入模板名称"
                        maxlength="50"
                    />
                </el-form-item>
            </el-form>
            <template #footer>
                <span class="dialog-footer">
                    <el-button @click="showCreateTemplateDialog = false">取消</el-button>
                    <el-button type="primary" @click="createNewTemplate">确定</el-button>
                </span>
            </template>
        </el-dialog>

        <!-- 登录对话框 -->
        <el-dialog
            v-model="showLoginDialog"
            width="500px"
            :show-close="false"
            :close-on-click-modal="true"
            :close-on-press-escape="false"
            :before-close="handleLoginDialogClose"
            class="login-dialog"
            top="5vh"
        >
            <template #header>
                <div></div>
            </template>
            <div class="login-dialog-content" @click.stop>
                <LoginView
                    @login-success="handleLoginSuccess"
                    @loading-change="loginLoading = $event"
                />
            </div>
        </el-dialog>

        <!-- 用户配置对话框 -->
        <UserConfig v-model="userConfigVisible" :user="configUser" />

        <!-- 全局配置对话框 -->
        <GlobalConfigView v-model="showGlobalConfigDialog" />
    </div>
</template>

<script setup lang="ts">
import { ref, onMounted, computed, nextTick, watch, onUnmounted } from 'vue'
import { v4 as uuidv4 } from 'uuid'
import { useAuthStore } from '../stores/auth'
import { useUserConfigStore } from '../stores/user_config'
import { useUtilsStore } from '../stores/utils'
import { useUploadStore } from '../stores/upload'
import { ElMessage, ElMessageBox } from 'element-plus'
import {
    ArrowDown,
    Plus,
    MoreFilled,
    UploadFilled,
    User,
    CircleCheck,
    Loading,
    Cloudy,
    Check,
    Edit,
    Delete,
    Setting,
    Close
} from '@element-plus/icons-vue'
import { open } from '@tauri-apps/plugin-dialog'
import { listen } from '@tauri-apps/api/event'
import LoginView from './LoginView.vue'
import TopicView from './TopicView.vue'
import SeasonView from './SeasonView.vue'
import UploadQueue from './UploadQueue.vue'
import UserConfig from './UserConfig.vue'
import GlobalConfigView from './GlobalConfigView.vue'

const authStore = useAuthStore()
const userConfigStore = useUserConfigStore()
const uploadStore = useUploadStore()
const utilsStore = useUtilsStore()

// 计算属性
const loginUsers = computed(() => authStore.loginUsers)
const userTemplates = computed(() => userConfigStore.userTemplates)
const typeList = computed(() => utilsStore.typelist)

// 封面显示URL
const coverDisplayUrl = ref<string>('')

// 响应式数据
const selectedUser = ref<any>(null)
const currentTemplateName = ref<string>('')
const showCreateTemplateDialog = ref(false)
const showLoginDialog = ref(false)
const showGlobalConfigDialog = ref(false)
const loginLoading = ref(false)
const uploading = ref(false)
const submitting = ref(false)
const autoSubmitting = ref(false)
const autoSubmitTimeout = ref<number | null>(null)
const lastSubmit = ref<string>('')
const inputVisible = ref(false)
const newTag = ref('')
const tagInputRef = ref()

// 卡片折叠状态
const cardCollapsed = ref({
    basic: false, // 基本信息
    tags: false, // 标签设置
    description: false, // 视频描述
    videos: false, // 视频文件
    advanced: false // 高级选项
})

// 模板名编辑相关
const isEditingTemplateName = ref(false)
const editingTemplateName = ref('')
const templateNameInputRef = ref()

// 拖拽状态
const isDragOver = ref(false)

// 文件编辑状态
const editingFileId = ref<string | null>(null)
const editingFileName = ref('')

// 内容容器引用
const contentWrapperRef = ref<HTMLElement | null>(null)

// 用户配置相关
const userConfigVisible = ref(false)
const configUser = ref<any>(null)

// 分区数据
const selectedCategory = ref<any>(null)
const selectedSubCategory = ref<any>(null)
const categoryPopoverVisible = ref(false)

// 表单数据
interface uploadForm {
    title: string,
    cover: string,
    copyright: number,
    source: string,
    aid?: number,
    tid: number,
    tag: string,
    desc: string,
    dynamic: string,
    videos:  any[],
    dtime?: number,
    open_subtitle: boolean,
    interactive: number,
    mission_id?: number,
    topic_id?: number,
    season_id?: number,
    section_id?: number,
    dolby: number,
    lossless_music: number,
    no_reprint: number,
    open_elec: number,
    up_selection_reply: number,
    up_close_reply: number,
    up_close_danmu: number,
    is_only_self: number
}

const currentForm = ref<uploadForm>({
    title: '',
    cover: '',
    copyright: 1,
    source: '',
    aid: undefined,
    tid: 0,
    tag: '',
    desc: '',
    dynamic: '',
    videos: [],
    dtime: undefined,
    open_subtitle: false,
    interactive: 0,
    mission_id: undefined,
    topic_id: undefined,
    season_id: undefined,
    section_id: undefined,
    dolby: 0,
    lossless_music: 0,
    no_reprint: 0,
    open_elec: 0,
    up_selection_reply: 0,
    up_close_reply: 0,
    up_close_danmu: 0,
    is_only_self: 0
})

const newTemplateForm = ref({
    userUid: null,
    name: '',
    templateType: 'blank', // 'blank' | 'bv'
    bvNumber: '',
    actionType: 'copy' // 'edit' | 'copy'
})

// 标签数据
const tags = ref<string[]>([])

// 当前模板配置
const currentTemplate = computed(() => {
    if (!selectedUser.value || !currentTemplateName.value) return null
    return userConfigStore.getUserTemplate(selectedUser.value.uid, currentTemplateName.value)
})

// 基础模板配置（用于检测是否有未保存的修改）
const baseTemplate = ref<any>(null)

// 日期选择器的计算属性 - 处理时间戳转换
const dtimeDate = computed({
    get() {
        return currentForm.value.dtime ? new Date(currentForm.value.dtime * 1000) : null
    },
    set(value: Date | null) {
        currentForm.value.dtime = value ? Math.floor(value.getTime() / 1000) : undefined
    }
})

// 检测是否有未保存的修改
const hasUnsavedChanges = computed(() => {
    // 如果没有基础模板或当前模板，则没有未保存的修改
    if (!baseTemplate.value || !currentTemplate.value) {
        return false
    }

    // 如果当前选择的用户或模板名为空，则没有未保存的修改
    if (!selectedUser.value || !currentTemplateName.value) {
        return false
    }

    // 构建当前表单数据
    const currentFormData = {
        title: currentForm.value.title,
        cover: currentForm.value.cover,
        copyright: currentForm.value.copyright,
        source: currentForm.value.source,
        tid: currentForm.value.tid,
        desc: currentForm.value.desc,
        dynamic: currentForm.value.dynamic,
        tag: currentForm.value.tag,
        videos: currentForm.value.videos,
        dtime: currentForm.value.dtime || undefined,
        open_subtitle: currentForm.value.open_subtitle,
        interactive: currentForm.value.interactive,
        mission_id: currentForm.value.mission_id,
        topic_id: currentForm.value.topic_id,
        season_id: currentForm.value.season_id,
        section_id: currentForm.value.section_id,
        dolby: currentForm.value.dolby,
        lossless_music: currentForm.value.lossless_music,
        no_reprint: currentForm.value.no_reprint,
        open_elec: currentForm.value.open_elec,
        up_selection_reply: currentForm.value.up_selection_reply,
        up_close_reply: currentForm.value.up_close_reply,
        up_close_danmu: currentForm.value.up_close_danmu,
        is_only_self: currentForm.value.is_only_self
    }

    // 对比关键字段是否有变化
    const result = Object.keys(currentFormData).some((key: string) => {
        const a = (currentFormData as any)[key]
        const b = baseTemplate.value[key]
        // treat undefined and null as equal
        if (
            (a === undefined || a === null || a === '') &&
            (b === undefined || b === null || b === '')
        )
            return false
        if (a === currentFormData.videos) {
            // 只比较id, title, desc, path, filename
            if ((a as any[]).length !== (b as any[]).length) return true
            if ((a as any[]).length === 0 && (b as any[]).length === 0) return false
            const videoA = (a as any[]).map((v: any) => ({
                id: v.id,
                title: v.title,
                desc: v.desc,
                path: v.path,
                filename: v.filename
            }))
            const videoB = (b as any[]).map((v: any) => ({
                id: v.id,
                title: v.title,
                desc: v.desc,
                path: v.path,
                filename: v.filename
            }))
            return JSON.stringify(videoA) !== JSON.stringify(videoB)
        }
        if (JSON.stringify(a) !== JSON.stringify(b)) {
            console.log(`Field "${key}" has changed: ${JSON.stringify(a)} !== ${JSON.stringify(b)}`)
        }
        return JSON.stringify(a) !== JSON.stringify(b)
    })

    return result
})

// 生命周期
// 监听封面变化，异步加载显示用的封面URL
watch(
    () => currentForm.value.cover,
    async (newCover: string) => {
        if (newCover && selectedUser.value) {
            try {
                const downloadedCover = await utilsStore.downloadCover(
                    selectedUser.value.uid,
                    newCover
                )
                coverDisplayUrl.value = downloadedCover || ''
            } catch (error) {
                console.error('Failed to download cover:', error)
                coverDisplayUrl.value = ''
            }
        } else {
            coverDisplayUrl.value = ''
        }
    }
)

// 监听标签变化，更新表单数据
watch(
    () => tags.value,
    (newTags: string[]) => {
        currentForm.value.tag = newTags.join(',')
    },
    { deep: true }
)

// 监听用户切换，重新加载封面
watch(
    () => selectedUser.value,
    async (newUser: any) => {
        if (currentForm.value.cover && newUser) {
            try {
                const downloadedCover = await utilsStore.downloadCover(
                    newUser.uid,
                    currentForm.value.cover
                )
                coverDisplayUrl.value = downloadedCover || ''
            } catch (error) {
                console.error('Failed to download cover:', error)
                coverDisplayUrl.value = ''
            }
        } else {
            coverDisplayUrl.value = ''
        }
    }
)

// 键盘快捷键清理函数
let keyboardCleanup: (() => void) | null = null

onMounted(async () => {
    await initializeData()
    await setupDragAndDrop()
    keyboardCleanup = await setupKeyboardShortcuts()

    // 禁用右键菜单刷新
    document.addEventListener('contextmenu', (event: MouseEvent) => {
        event.preventDefault()
    })
})

// 在组件卸载时清理
onUnmounted(() => {
    if (keyboardCleanup) {
        keyboardCleanup()
    }

    // 清理自动提交的 timeout
    if (autoSubmitTimeout.value) {
        clearTimeout(autoSubmitTimeout.value)
        autoSubmitTimeout.value = null
    }
})

// 初始化数据
const initializeData = async () => {
    try {
        // 获取登录用户
        await authStore.getLoginUsers()

        // 构建用户模板列表
        if (loginUsers.value.length > 0) {
            await utilsStore.initTypeList(loginUsers.value[0].uid)
            await utilsStore.initTopicList(loginUsers.value[0].uid)
            await userConfigStore.buildUserTemplates(loginUsers.value)
            await uploadStore.getUploadQueue()
            setInterval(() => {
                uploadStore.getUploadQueue()
                for (const task of uploadStore.uploadQueue) {
                    if (task.status === 'Completed') {
                        const video = currentForm.value.videos.find(v => v.id === task.video?.id)
                        if (video && video.filename !== task.video?.filename) {
                            video.filename = task.video.filename
                            video.path = task.video.path
                            video.complete = true
                            saveTemplate()
                            userConfigStore.saveConfig()
                        }
                    }
                }
            }, 1000) // 每5秒更新一次上传队列
        }
    } catch (error) {
        console.error('初始化数据失败: ', error)
        ElMessage.error(`'初始化数据失败: ${error}'`)
    }
}

// 设置拖拽功能
const setupDragAndDrop = async () => {
    try {
        // 监听文件拖拽事件
        await listen('tauri://drag-drop', async event => {
            const videos = event.payload as string[]
            isDragOver.value = false
            await handleDroppedFiles(videos)
        })

        // 监听拖拽悬停事件
        await listen('tauri://drag-over', event => {
            console.log('文件拖拽悬停:', event.payload)
            isDragOver.value = true
        })

        // 监听拖拽取消事件
        await listen('tauri://drag-leave', () => {
            console.log('文件拖拽取消')
            isDragOver.value = false
        })
    } catch (error) {
        console.error('设置拖拽功能失败: ', error)
        ElMessage.error(`'设置拖拽功能失败: ${error}'`)
    }
}

// 设置键盘快捷键
const setupKeyboardShortcuts = async () => {
    const handleKeydown = (event: KeyboardEvent) => {
        // 禁用 F5 刷新
        if (event.key === 'F5') {
            event.preventDefault()
            return
        }

        // // 禁用 Ctrl+R 刷新
        // if (event.ctrlKey && event.key === 'r') {
        //     event.preventDefault()
        //     return
        // }

        // Ctrl+S 保存模板
        if (event.ctrlKey && event.key === 's') {
            event.preventDefault()
            if (selectedUser.value && currentTemplateName.value) {
                saveTemplate()
            }
        }
    }

    document.addEventListener('keydown', handleKeydown)

    // 返回清理函数
    return () => {
        document.removeEventListener('keydown', handleKeydown)
    }
}

// 切换卡片折叠状态
const toggleCardCollapsed = (cardKey: keyof typeof cardCollapsed.value) => {
    cardCollapsed.value[cardKey] = !cardCollapsed.value[cardKey]
}

const addVideoToCurrentForm = async (videoPath: string) => {
    // 从路径中提取文件名
    const videoBaseName = videoPath.split(/[/\\]/).pop() || videoPath
    const videoNameWOExtension = videoBaseName.replace(/\.[^/.]+$/, '')
    const videoExt = videoBaseName.split('.').pop()?.toLowerCase() || ''

    const extFilter = [
        'mp4',
        'flv',
        'avi',
        'wmv',
        'mov',
        'webm',
        'mpeg4',
        'ts',
        'mpg',
        'rm',
        'rmvb',
        'mkv',
        'm4v'
    ]

    if (videoExt && !extFilter.includes(videoExt)) {
        return 0 // 不支持的格式，跳过添加
    }

    // 检查文件是否已经存在
    const existingFile = currentForm.value.videos.find(f => f.path === videoPath)
    if (existingFile) {
        return 0 // 跳过已存在的文件
    }

    // 添加到currentForm.videos
    const videoId = uuidv4()
    currentForm.value.videos.push({
        id: videoId,
        filename: videoBaseName, // 使用完整的文件路径
        title: videoNameWOExtension, // 去除扩展名作为标题
        desc: '',
        path: videoPath, // 保存完整路径
        complete: false
    })

    // 检查是否启用自动添加到上传队列
    if (userConfigStore.configRoot?.auto_upload && selectedUser.value) {
        try {
            // 自动创建上传任务
            const video = currentForm.value.videos[currentForm.value.videos.length - 1]
            await uploadStore.createUploadTask(selectedUser.value.uid, [video])
            console.log(`自动添加文件到上传队列: ${videoBaseName}`)

            // 如果同时启用自动开始，则自动开始任务
            if (userConfigStore.configRoot?.auto_start) {
                // 延迟一下让任务先添加到队列
                setTimeout(async () => {
                    try {
                        await autoStartWaitingTasks()
                    } catch (error) {
                        console.error('自动开始任务失败:', error)
                    }
                }, 500)
            }
        } catch (error) {
            console.error('自动添加到上传队列失败:', error)
        }
    }

    return 1
}

// 处理拖拽文件
const handleDroppedFiles = async (videoFiles: any) => {
    // 检查是否有选中的用户和模板
    if (!selectedUser.value || !currentTemplateName.value) {
        ElMessage.warning('请先选择用户和模板后再拖拽文件')
        return
    }

    // 添加视频文件到当前模板
    let addedCount = 0
    for (const videoPath of videoFiles.paths) {
        addedCount += await addVideoToCurrentForm(videoPath)
    }

    if (addedCount > 0) {
        ElMessage.success(`成功添加 ${addedCount} 个视频文件`)
    } else {
        ElMessage.info('所有文件都已存在，未添加新文件')
    }
}

// 处理登录成功
const handleLoginSuccess = async () => {
    showLoginDialog.value = false
    ElMessage.success('登录成功')

    await userConfigStore.saveConfig()
    // 刷新所有数据
    await refreshAllData()
}

// 处理登录对话框关闭
const handleLoginDialogClose = async (done: () => void) => {
    if (loginLoading.value) {
        ElMessage.warning('登录过程中无法取消')
        return
    }

    try {
        await ElMessageBox.confirm('确定要取消登录吗？', '提示', {
            confirmButtonText: '确定',
            cancelButtonText: '继续登录',
            type: 'warning'
        })
        done()
    } catch (error) {
        // 用户点击了取消，不关闭对话框
    }
}

// 切换用户展开状态
const toggleUserExpanded = (userUid: number) => {
    userConfigStore.toggleUserExpanded(userUid)
}

// 选择模板
const selectTemplate = async (user: any, templateName: string) => {
    if (autoSubmitTimeout.value) {
        try {
            await ElMessageBox.confirm(
                '正在自动提交中，切换模板将导致自动提交失败',
                '确认切换模板',
                {
                    confirmButtonText: '确认并切换',
                    cancelButtonText: '不切换',
                    distinguishCancelAndClose: true,
                    type: 'warning'
                }
            )
        } catch (action) {
            return
        }
    }

    // 检查是否有未保存的修改（但只在真正有已选择模板的情况下检查）
    if (hasUnsavedChanges.value && selectedUser.value && currentTemplateName.value) {
        try {
            await ElMessageBox.confirm('当前模板有未保存的修改，是否保存？', '确认切换模板', {
                confirmButtonText: '保存并切换',
                cancelButtonText: '不保存',
                distinguishCancelAndClose: true,
                type: 'warning'
            })

            // 用户选择保存
            await saveTemplate()
        } catch (action) {
            if (action === 'cancel') {
                // 用户选择不保存，继续切换
            } else {
                // 用户取消操作，不切换模板
                return
            }
        }
    }

    // 立即清空基础模板
    baseTemplate.value = null

    // 清理自动提交状态
    if (autoSubmitTimeout.value) {
        clearTimeout(autoSubmitTimeout.value)
        autoSubmitTimeout.value = null
    }
    autoSubmitting.value = false
    lastSubmit.value = ''

    selectedUser.value = user
    currentTemplateName.value = templateName

    // 滚动到顶部
    nextTick(() => {
        if (contentWrapperRef.value) {
            contentWrapperRef.value.scrollTop = 0
        }
    })

    // 加载模板数据到表单
    await loadTemplateToForm()
}

// 加载模板数据到表单
const loadTemplateToForm = async () => {
    try {
        // 如果没有模板，则初始化为空白表单
        if (!currentTemplate.value) {
            currentForm.value = {
                title: '',
                cover: '',
                copyright: 1,
                source: '',
                aid: undefined,
                tid: 0,
                desc: '',
                dynamic: '',
                videos: [],
                tag: '',
                dtime: undefined,
                open_subtitle: false,
                interactive: 0,
                mission_id: undefined,
                topic_id: undefined,
                season_id: undefined,
                section_id: undefined,
                dolby: 0,
                lossless_music: 0,
                no_reprint: 0,
                open_elec: 0,
                up_selection_reply: 0,
                up_close_reply: 0,
                up_close_danmu: 0,
                is_only_self: 0
            }

            // 清空标签
            tags.value = []

            // 清空分区选择
            selectedCategory.value = null
            selectedSubCategory.value = null

            // 重置基础模板
            baseTemplate.value = null

            // 等待所有更新完成
            await nextTick()

            return
        }

        const template = currentTemplate.value
        currentForm.value = {
            title: template.title || '',
            cover: template.cover || '',
            copyright: template.copyright || 1,
            source: template.source || '',
            aid: template.aid || undefined,
            tid: template.tid || 0,
            desc: template.desc || '',
            dynamic: template.dynamic || '',
            videos: template.videos || [],
            tag: template.tag,
            dtime: template.dtime || undefined,
            open_subtitle: template.open_subtitle || false,
            interactive: template.interactive || 0,
            mission_id: template.mission_id,
            topic_id: template.topic_id,
            season_id: template.season_id || undefined,
            section_id: template.section_id || undefined,
            dolby: template.dolby || 0,
            lossless_music: template.lossless_music || 0,
            no_reprint: template.no_reprint || 0,
            open_elec: template.open_elec || 0,
            up_selection_reply: template.up_selection_reply || 0,
            up_close_reply: template.up_close_reply || 0,
            up_close_danmu: template.up_close_danmu || 0,
            is_only_self: template.is_only_self || 0
        }

        // 解析标签
        tags.value = template.tag ? template.tag.split(',').filter(tag => tag.trim()) : []

        // 设置选中的分区
        if (template.tid) {
            setSelectedCategoryByTid(template.tid)
        } else {
            // 如果没有分区信息，清空分区选择
            selectedCategory.value = null
            selectedSubCategory.value = null
        }

        // 等待所有表单数据更新完成
        await nextTick()

        // 保存基础模板状态（深拷贝）
        baseTemplate.value = JSON.parse(JSON.stringify(template))
    } catch (error) {
        console.error('加载模板失败:', error)
        ElMessage.error(`加载模板失败: ${error}`)
    }
}

// 处理模板命令
const handleTemplateCommand = async (command: string, user: any, template: any) => {
    switch (command) {
        case 'duplicate':
            try {
                const newName = `${template.name}_副本`
                await userConfigStore.duplicateUserTemplate(user.uid, template.name, newName)
                ElMessage.success('模板复制成功')
            } catch (error) {
                console.error('复制模板失败: ', error)
                ElMessage.error(`'复制模板失败: ${error}'`)
            }
            break

        case 'rename':
            try {
                const { value: newName } = await ElMessageBox.prompt(
                    '请输入新的模板名称',
                    '重命名模板',
                    {
                        confirmButtonText: '确定',
                        cancelButtonText: '取消',
                        inputPlaceholder: '请输入模板名称',
                        inputValue: template.name,
                        inputValidator: (value: string) => {
                            if (!value || !value.trim()) {
                                return '模板名称不能为空'
                            }
                            if (value.trim() === template.name) {
                                return '新名称不能与原名称相同'
                            }
                            return true
                        }
                    }
                )

                const trimmedName = newName.trim()

                // 检查是否已存在同名模板
                const existingTemplate = userConfigStore.getUserTemplate(user.uid, trimmedName)
                if (existingTemplate) {
                    ElMessage.error('该名称的模板已存在，请使用其他名称')
                    return
                }

                // 获取原模板配置
                const originalTemplate = userConfigStore.getUserTemplate(user.uid, template.name)
                if (!originalTemplate) {
                    ElMessage.error('原模板不存在')
                    return
                }

                // 先添加新模板
                await userConfigStore.addUserTemplate(user.uid, trimmedName, originalTemplate)

                // 再删除原模板
                await userConfigStore.removeUserTemplate(user.uid, template.name)

                // 如果重命名的是当前选中的模板，更新选择
                if (
                    selectedUser.value?.uid === user.uid &&
                    currentTemplateName.value === template.name
                ) {
                    currentTemplateName.value = trimmedName
                    // 更新基础模板状态
                    if (baseTemplate.value) {
                        baseTemplate.value = JSON.parse(JSON.stringify(originalTemplate))
                    }
                }

                ElMessage.success('模板重命名成功')
            } catch (error) {
                if (error !== 'cancel') {
                    console.error('重命名模板失败: ', error)
                    ElMessage.error(`'重命名模板失败: ${error}'`)
                }
            }
            break

        case 'delete':
            try {
                const template_name = template.name || currentTemplateName.value
                await ElMessageBox.confirm(`确定要删除模板"${template_name}"吗？`, '确认删除', {
                    confirmButtonText: '确定',
                    cancelButtonText: '取消',
                    type: 'warning'
                })

                await userConfigStore.removeUserTemplate(user.uid, template_name)

                // 如果删除的是当前选中的模板，清空选择
                if (
                    selectedUser.value?.uid === user.uid &&
                    currentTemplateName.value === template_name
                ) {
                    currentTemplateName.value = ''
                    selectedUser.value = null
                    baseTemplate.value = null
                }

                ElMessage.success('模板删除成功')
            } catch (error) {
                if (error !== 'cancel') {
                    console.error('删除模板失败: ', error)
                    ElMessage.error(`'删除模板失败: ${error}'`)
                }
            }
            break
    }
}

// 创建新模板
const createNewTemplate = async () => {
    const targetUserUid = newTemplateForm.value.userUid || selectedUser.value?.uid
    if (!targetUserUid) {
        ElMessage.error('请选择用户')
        return
    }

    try {
        if (newTemplateForm.value.templateType === 'blank') {
            // 空白模板
            if (!newTemplateForm.value.name.trim()) {
                ElMessage.error('请输入模板名称')
                return
            }

            const templateName = newTemplateForm.value.name.trim()
            await userConfigStore.addUserTemplate(targetUserUid, templateName)

            // 自动选择新创建的模板
            const targetUser = loginUsers.value.find(user => user.uid === targetUserUid)
            if (targetUser) {
                selectedUser.value = targetUser
                currentTemplateName.value = templateName

                // 滚动到顶部
                nextTick(() => {
                    if (contentWrapperRef.value) {
                        contentWrapperRef.value.scrollTop = 0
                    }
                })

                await loadTemplateToForm()
            }

            ElMessage.success('空白模板创建成功')
        } else if (newTemplateForm.value.templateType === 'bv') {
            // BV/AV号模板
            if (!newTemplateForm.value.bvNumber.trim()) {
                ElMessage.error('请输入BV号或AV号')
                return
            }

            const bvNumber = newTemplateForm.value.bvNumber.trim()
            const actionType = newTemplateForm.value.actionType
            const templateName = newTemplateForm.value.name.trim() || `编辑_${bvNumber}`

            const isEdit = actionType === 'edit'
            await createTemplateFromBV(targetUserUid, bvNumber, templateName, isEdit)

            // 自动选择新创建的模板
            const targetUser = loginUsers.value.find(user => user.uid === targetUserUid)
            if (targetUser) {
                selectedUser.value = targetUser
                currentTemplateName.value = templateName

                // 滚动到顶部
                nextTick(() => {
                    if (contentWrapperRef.value) {
                        contentWrapperRef.value.scrollTop = 0
                    }
                })

                await loadTemplateToForm()
            }

            ElMessage.success('基于稿件创建模板成功')
        }

        showCreateTemplateDialog.value = false
        newTemplateForm.value = {
            userUid: null,
            name: '',
            templateType: 'blank',
            bvNumber: '',
            actionType: 'copy'
        }
    } catch (error) {
        console.error('创建模板失败:', error)
        // 提供更详细的错误信息
        let errorMessage = '创建模板失败'
        if (error instanceof Error) {
            errorMessage = `创建模板失败: ${error.message}`
        } else if (typeof error === 'string') {
            errorMessage = `创建模板失败: ${error}`
        }
        ElMessage.error(errorMessage)
    }
}

// 根据BV号创建模板
const createTemplateFromBV = async (
    userUid: number,
    bvNumber: string,
    templateName: string,
    isEdit: boolean
) => {
    try {
        console.log(`从BV号 ${bvNumber} 创建模板: ${templateName}`)

        const newForm = await utilsStore.getVideoDetail(userUid, bvNumber) as uploadForm

        for (const video of newForm.videos) {
            video.id = video.filename
            video.path = ''
        }

        const newTemplate = buildTemplateFromUploadFrom(newForm)

        if (!isEdit) {
            newTemplate.aid = undefined
        }

        await userConfigStore.addUserTemplate(userUid, templateName, newTemplate)
    } catch (error) {
        console.error('从BV号创建模板失败: ', error)
        ElMessage.error(`'从BV号创建模板失败: ${error}'`)
        throw error
    }
}

const buildTemplateFromUploadFrom = (form: uploadForm) => {
    return {
            ...userConfigStore.createDefaultTemplate(),
            ...currentTemplate.value,
            title: form.title,
            cover: form.cover,
            copyright: form.copyright,
            source: form.source,
            aid: form.aid,
            tid: form.tid,
            desc: form.desc,
            dynamic: form.dynamic,
            tag: form.tag,
            videos: form.videos,
            dtime: form.dtime || undefined,
            open_subtitle: form.open_subtitle,
            interactive: form.interactive,
            mission_id: form.mission_id,
            topic_id: form.topic_id,
            season_id: form.season_id,
            section_id: form.section_id,
            dolby: form.dolby,
            lossless_music: form.lossless_music,
            no_reprint: form.no_reprint,
            open_elec: form.open_elec,
            up_selection_reply: form.up_selection_reply,
            up_close_reply: form.up_close_reply,
            up_close_danmu: form.up_close_danmu,
            is_only_self: form.is_only_self
        }

}

// 保存模板
const saveTemplate = async () => {
    if (!selectedUser.value || !currentTemplateName.value) {
        ElMessage.error('请先选择模板')
        return
    }

    try {
        const templateConfig = buildTemplateFromUploadFrom(currentForm.value)

        await userConfigStore.updateUserTemplate(
            selectedUser.value.uid,
            currentTemplateName.value,
            templateConfig
        )

        // 更新基础模板状态
        baseTemplate.value = JSON.parse(JSON.stringify(templateConfig))

        ElMessage.success('模板保存成功')
    } catch (error) {
        console.error('保存模板失败: ', error)
        ElMessage.error(`'保存模板失败: ${error}'`)
    }
}

// 标签相关
const showTagInput = () => {
    inputVisible.value = true
    nextTick(() => {
        tagInputRef.value?.focus()
    })
}

const addTag = () => {
    const tag = newTag.value.trim()
    if (tag && !tags.value.includes(tag)) {
        tags.value.push(tag)
    }
    newTag.value = ''
    inputVisible.value = false
}

const removeTag = (tag: string) => {
    const index = tags.value.indexOf(tag)
    if (index > -1) {
        tags.value.splice(index, 1)
    }
}

// 分区选择相关
const onCategoryChange = (categoryId: number) => {
    const category = typeList.value.find(item => item.id === categoryId)
    selectedCategory.value = category
    selectedSubCategory.value = null
    currentForm.value.tid = 0
}

const onSubCategoryChange = (subCategoryId: number) => {
    if (selectedCategory.value && selectedCategory.value.children) {
        const subCategory = selectedCategory.value.children.find(
            (item: any) => item.id === subCategoryId
        )
        selectedSubCategory.value = subCategory
        currentForm.value.tid = subCategoryId
        // 选择子分区后关闭popover
        categoryPopoverVisible.value = false
    }
}

// 根据tid设置选中的分区
const setSelectedCategoryByTid = (tid: number) => {
    for (const category of typeList.value) {
        if (category.children) {
            const subCategory = category.children.find((item: any) => item.id === tid)
            if (subCategory) {
                selectedCategory.value = category
                selectedSubCategory.value = subCategory
                return
            }
        }
    }
}

// 选择封面
const selectCoverWithTauri = async () => {
    try {
        const selected = await open({
            multiple: false,
            filters: [
                {
                    name: 'Image',
                    extensions: ['jpg', 'jpeg', 'png', 'pjp', 'pjpeg', 'jiff', 'gif']
                }
            ]
        })

        if (!selected || selected.length === 0) {
            ElMessage.warning('未选择任何封面文件')
            return
        }

        if (selectedUser.value && currentTemplate.value && currentForm.value) {
            const url = await utilsStore.uploadCover(selectedUser.value.uid, selected)
            if (url) {
                currentTemplate.value.cover = url
                currentForm.value.cover = url
            } else {
                throw new Error('封面上传失败')
            }
        } else {
            ElMessage.error('请先选择用户和模板')
        }
    } catch (error) {
        console.error('封面选择失败: ', error)
        ElMessage.error(`'封面选择失败: ${error}'`)
        return
    }
}

// 使用 Tauri 文件对话框选择视频文件
const selectVideoWithTauri = async () => {
    try {
        const selected = await open({
            multiple: true,
            filters: [
                {
                    name: 'Video',
                    extensions: [
                        'mp4',
                        'flv',
                        'avi',
                        'wmv',
                        'mov',
                        'webm',
                        'mpeg4',
                        'ts',
                        'mpg',
                        'rm',
                        'rmvb',
                        'mkv',
                        'm4v'
                    ]
                }
            ]
        })

        var added = 0

        if (selected && Array.isArray(selected)) {
            for (const videoPath of selected) {
                added += await addVideoToCurrentForm(videoPath)
            }

            ElMessage.success(`已选择 ${added} 个文件`)
        } else if (typeof selected === 'string') {
            added += await addVideoToCurrentForm(selected)
            ElMessage.success(`已选择 ${added} 个文件`)
        }
    } catch (error) {
        console.error('文件选择失败: ', error)
        ElMessage.error(`'文件选择失败: ${error}'`)
    }
}

// 清空所有文件
const clearAllVideos = async () => {
    if (!currentForm.value.videos || currentForm.value.videos.length === 0) {
        return
    }

    const videoCount = currentForm.value.videos.length
    const videoText = videoCount === 1 ? '1 个文件' : `${videoCount} 个文件`

    try {
        await ElMessageBox.confirm(
            `确定要清空所有已选择的 ${videoText} 吗？此操作不可撤销。`,
            '确认清空文件',
            {
                confirmButtonText: '确定清空',
                cancelButtonText: '取消',
                type: 'warning',
                dangerouslyUseHTMLString: false
            }
        )

        // 取消所有对应的上传任务
        const videoIds = currentForm.value.videos.map(video => video.id)
        const correspondingTasks = uploadStore.uploadQueue.filter(task =>
            videoIds.includes(task.video?.id)
        )

        for (const task of correspondingTasks) {
            try {
                await uploadStore.cancelUpload(task.id)
                console.log(`已取消对应的上传任务: ${task.id}`)
            } catch (error) {
                console.error('取消上传任务失败:', error)
                // 继续处理其他任务
            }
        }

        // 清空视频文件列表
        currentForm.value.videos = []
        ElMessage.success(`已清空 ${videoText}`)
    } catch {
        // 用户取消了操作
    }
}

// 文件管理相关
const startEditFileName = (videoId: string, currentTitle: string) => {
    editingFileId.value = videoId
    editingFileName.value = currentTitle
}

const saveFileName = (videoId: string) => {
    const newTitle = editingFileName.value.trim()
    if (!newTitle) {
        ElMessage.error('文件名不能为空')
        return
    }

    // 更新currentForm.videos中的标题
    const videoIndex = currentForm.value.videos.findIndex(f => f.id === videoId)
    if (videoIndex > -1) {
        currentForm.value.videos[videoIndex].title = newTitle
    }

    // 清空编辑状态
    editingFileId.value = null
    editingFileName.value = ''
}

const cancelEditFileName = () => {
    editingFileId.value = null
    editingFileName.value = ''
}

const removeUploadedFile = async (videoId: string) => {
    const videoIndex = currentForm.value.videos.findIndex(f => f.id === videoId)
    if (videoIndex > -1) {
        const video = currentForm.value.videos[videoIndex]

        try {
            // 添加确认弹窗
            await ElMessageBox.confirm(
                `确定要删除视频文件"${video.title}"吗？此操作不可撤销。`,
                '确认删除文件',
                {
                    confirmButtonText: '确定删除',
                    cancelButtonText: '取消',
                    type: 'warning'
                }
            )

            // 先查找并取消对应的上传任务
            const correspondingTask = uploadStore.uploadQueue.find(
                task => task.video?.id === videoId
            )
            if (correspondingTask) {
                try {
                    await uploadStore.cancelUpload(correspondingTask.id)
                    console.log(`已取消对应的上传任务: ${correspondingTask.id}`)
                } catch (error) {
                    console.error('取消上传任务失败:', error)
                    // 即使取消失败，仍然继续删除文件
                }
            }

            // 删除视频文件
            currentForm.value.videos.splice(videoIndex, 1)

            ElMessage.success('文件删除成功')
        } catch (error) {
            // 如果用户取消了确认框，不显示错误消息
            if (error !== 'cancel') {
                console.error('删除文件失败:', error)
                ElMessage.error(`删除文件失败: ${error}`)
            }
        }
    }
}

// 上传相关
const createUpload = async () => {
    // 检查是否有文件可上传
    const hasUploadedFiles = currentForm.value.videos && currentForm.value.videos.length > 0

    if (!hasUploadedFiles) {
        ElMessage.error('请先选择视频文件')
        return
    }

    if (!selectedUser.value) {
        ElMessage.error('请先选择用户')
        return
    }

    uploading.value = true
    try {
        console.log('开始上传文件:', currentForm.value.videos)
        // 确保传递的是正确格式的数组
        const num_added = await uploadStore.createUploadTask(
            selectedUser.value.uid,
            currentForm.value.videos
        )
        ElMessage.success(`添加 ${num_added} 个文件到上传队列`)

        // 如果启用自动开始，则自动开始任务
        if (userConfigStore.configRoot?.auto_start) {
            setTimeout(async () => {
                try {
                    await autoStartWaitingTasks()
                } catch (error) {
                    console.error('自动开始任务失败:', error)
                }
            }, 500)
        }
    } catch (error) {
        console.error('上传失败: ', error)
        ElMessage.error(`上传失败: ${error}`)
    } finally {
        uploading.value = false
    }
}

// 自动开始待处理的任务
const autoStartWaitingTasks = async () => {
    if (!userConfigStore.configRoot?.auto_start) {
        return
    }

    // 刷新上传队列获取最新状态
    await uploadStore.getUploadQueue()

    // 获取所有待处理的任务
    const pendingTasks = uploadStore.uploadQueue.filter(task => task.status === 'Waiting')

    for (const task of pendingTasks) {
        try {
            await uploadStore.startUpload(task.id)
            console.log(`自动开始任务: ${task.id}`)
        } catch (error) {
            console.error(`自动开始任务失败 ${task.id}:`, error)
            // 继续处理下一个任务
        }
    }
}

// 检查是否所有文件都已上传完成
const allFilesUploaded = computed(() => {
    if (!currentForm.value.videos || currentForm.value.videos.length === 0) {
        return false
    }
    return currentForm.value.videos.every(video => video.complete)
})

// 提交视频
const submitTemplate = async (from_timeout: boolean) => {
    if (!currentTemplateName.value) {
        ElMessage.error('请选择模板')
        return
    }

    if (!allFilesUploaded.value) {
        if (!autoSubmitting.value) {
            // 首次点击，开始自动提交
            autoSubmitting.value = true
            autoSubmitTimeout.value = setTimeout(async () => {
                await submitTemplate(true)
            }, 500)
            ElMessage.info('已启动自动提交，上传完成后将自动提交')
        } else {
            if (from_timeout) {
                autoSubmitTimeout.value = setTimeout(async () => {
                    await submitTemplate(true)
                }, 500)
            } else {
                // 第二次点击，取消自动提交
                if (autoSubmitTimeout.value) {
                    clearTimeout(autoSubmitTimeout.value)
                    autoSubmitTimeout.value = null
                }
                autoSubmitting.value = false
                ElMessage.info('已取消自动提交')
            }
        }
        return
    }

    for (const video of currentForm.value.videos) {
        if (video.path !== '') {
            autoSubmitTimeout.value = setTimeout(async () => {
                await submitTemplate(true)
            }, 1000)
            return
        }
    }

    submitting.value = true
    try {
        await uploadStore.submitTemplate(selectedUser.value.uid, currentForm.value)
        lastSubmit.value = new Date().toLocaleString()
        ElMessage.success('视频提交成功')
    } catch (error) {
        console.error('视频提交失败: ', error)
        ElMessage.error(`视频提交失败: ${error}`)
    } finally {
        submitting.value = false
        autoSubmitting.value = false
        if (autoSubmitTimeout.value) {
            clearTimeout(autoSubmitTimeout.value)
            autoSubmitTimeout.value = null
        }
    }
}

const updateVideo = (videos: any[]) => {
    for (let i = 0; i < videos.length; i++) {
        if (videos[i].id == videos[i].filename || !videos[i].path) {
            videos[i].complete = true
        } else {
            const task = uploadStore.getUploadTask(videos[i].id)
            if (task) {
                videos[i].complete = task.status === 'Completed'
                videos[i].totalSize = task.total_size || 0
                videos[i].speed = task.speed || 0
                videos[i].progress = task.progress || 0
            } else {
                videos[i].complete = false
                videos[i].totalSize = 0
                videos[i].speed = 0
                videos[i].progress = 0
            }
        }
    }
    return videos
}

const formatUploadProgress = (video: any): string => {
    if (video.complete) return '100'
    if (!video || video.progress === undefined) return '0'
    if (video.progress >= 100) return '100'

    return `${video.progress.toFixed(2)}`
}

// 格式化上传速率
const formatUploadSpeed = (video: any): string => {
    if (!video || video.speed === undefined) return '0 B/s'

    const units = ['B/s', 'KB/s', 'MB/s', 'GB/s']
    let size = video.speed
    let unitIndex = 0

    while (size >= 1024 && unitIndex < units.length - 1) {
        size /= 1024
        unitIndex++
    }

    return `${size.toFixed(unitIndex === 0 ? 0 : 1)} ${units[unitIndex]}`
}

// 模板名编辑相关函数
const startEditTemplateName = () => {
    isEditingTemplateName.value = true
    editingTemplateName.value = currentTemplateName.value
    nextTick(() => {
        templateNameInputRef.value?.focus()
    })
}

const saveTemplateName = async () => {
    const newName = editingTemplateName.value.trim()

    if (!newName) {
        ElMessage.error('模板名称不能为空')
        cancelEditTemplateName()
        return
    }

    if (newName === currentTemplateName.value) {
        cancelEditTemplateName()
        return
    }

    if (!selectedUser.value) {
        ElMessage.error('未选择用户')
        cancelEditTemplateName()
        return
    }

    try {
        // 检查是否已存在同名模板
        const existingTemplate = userConfigStore.getUserTemplate(selectedUser.value.uid, newName)
        if (existingTemplate) {
            ElMessage.error('该名称的模板已存在，请使用其他名称')
            return
        }

        // 获取原模板配置
        const originalTemplate = userConfigStore.getUserTemplate(
            selectedUser.value.uid,
            currentTemplateName.value
        )
        if (!originalTemplate) {
            ElMessage.error('原模板不存在')
            cancelEditTemplateName()
            return
        }

        // 先添加新模板
        await userConfigStore.addUserTemplate(selectedUser.value.uid, newName, originalTemplate)

        // 再删除原模板
        await userConfigStore.removeUserTemplate(selectedUser.value.uid, currentTemplateName.value)

        // 更新当前选择
        currentTemplateName.value = newName

        // 更新基础模板状态
        if (baseTemplate.value) {
            baseTemplate.value = JSON.parse(JSON.stringify(originalTemplate))
        }

        ElMessage.success('模板重命名成功')
        isEditingTemplateName.value = false
    } catch (error) {
        console.error('重命名模板失败: ', error)
        ElMessage.error(`重命名模板失败: ${error}`)
        cancelEditTemplateName()
    }
}

const cancelEditTemplateName = () => {
    isEditingTemplateName.value = false
    editingTemplateName.value = ''
}

// 用户配置相关方法
const openUserConfig = (user: any) => {
    configUser.value = user
    userConfigVisible.value = true
}

// 检查用户是否有上传任务
const isUserHasUploadTasks = (uid: number) => {
    return uploadStore.uploadQueue.some((task: any) => task.user?.uid === uid)
}

// 处理用户登出
const handleLogoutUser = async (uid: number) => {
    // 如果用户有上传任务，不允许登出
    if (isUserHasUploadTasks(uid)) {
        return
    }

    try {
        // 添加确认提示框
        await ElMessageBox.confirm('确定要退出该用户吗？退出后会删除所有本地数据。', '退出确认', {
            confirmButtonText: '确认退出',
            cancelButtonText: '取消',
            type: 'warning'
        })

        const success = await authStore.logoutUser(uid)
        if (success) {
            ElMessage.success('用户已登出')
            // 刷新前端数据
            await refreshAllData()
        } else {
            ElMessage.error('登出失败')
        }
    } catch (error) {
        // 如果用户取消了确认框，error会是'cancel'，不需要显示错误
        if (error !== 'cancel') {
            console.error('登出用户失败:', error)
            ElMessage.error(`登出失败: ${error}`)
        }
    }
}

// 刷新所有数据的方法
const refreshAllData = async () => {
    try {
        // 重新获取登录用户
        await authStore.getLoginUsers()
        // 重新构建用户模板
        await userConfigStore.buildUserTemplates(authStore.loginUsers)
        // 重新加载用户配置
        await userConfigStore.loadConfig()
        // 重写
        await userConfigStore.saveConfig()
    } catch (error) {
        console.error('刷新数据失败:', error)
    }
}
</script>

<style scoped>
.main-view {
    height: 100vh;
    display: flex;
    flex-direction: column;
    overflow: hidden;
}

.header {
    background: #fff;
    border-bottom: 1px solid #e4e7ed;
    padding: 0 20px;
    position: sticky;
    top: 0;
    z-index: 100;
    flex-shrink: 0;
}

.header-content {
    display: flex;
    justify-content: space-between;
    align-items: center;
    height: 100%;
}

.app-title {
    margin: 0;
    color: #303133;
}

.header-right {
    display: flex;
    align-items: center;
    gap: 20px;
}

.user-info {
    display: flex;
    align-items: center;
    gap: 8px;
    color: #606266;
    font-size: 14px;
}

.global-config-btn {
    margin-right: 12px;
}

.main-container {
    flex: 1;
    overflow: hidden;
}

.sidebar {
    background: #f5f7fa;
    border-right: 1px solid #e4e7ed;
    padding: 20px;
    display: flex;
    flex-direction: column;
    overflow: hidden;
}

.sidebar-content {
    flex: 1;
    overflow-y: auto;
    scrollbar-width: thin;
    scrollbar-color: #c1c1c1 transparent;
}

.sidebar-content::-webkit-scrollbar {
    width: 6px;
}

.sidebar-content::-webkit-scrollbar-track {
    background: transparent;
}

.sidebar-content::-webkit-scrollbar-thumb {
    background-color: #c1c1c1;
    border-radius: 3px;
}

.sidebar-content::-webkit-scrollbar-thumb:hover {
    background-color: #a8a8a8;
}

.sidebar-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 20px;
}

.sidebar-header h3 {
    margin: 0;
    color: #303133;
}

.header-buttons {
    display: flex;
    gap: 8px;
}

.user-section {
    margin-bottom: 10px;
}

.user-header {
    display: flex;
    align-items: center;
    padding: 10px;
    background: #fff;
    border-radius: 6px;
    cursor: pointer;
    transition: background-color 0.3s;
}

.user-header:hover {
    background: #ecf5ff;
}

.user-avatar {
    margin-right: 10px;
}

.user-name {
    flex: 1;
    font-weight: 500;
}

.template-count-badge {
    margin-right: 10px;
}

.template-count-badge :deep(.el-badge__content) {
    background-color: #909399 !important;
    color: #ffffff !important;
    border: none !important;
}

.expand-icon {
    transition: transform 0.3s;
}

.expand-icon.expanded {
    transform: rotate(180deg);
}

.config-icon {
    color: #909399;
    cursor: pointer;
    margin-left: 8px;
    margin-right: 4px;
    transition: color 0.3s;
}

.config-icon:hover {
    color: #409eff;
}

.logout-icon {
    color: #f56c6c;
    cursor: pointer;
    margin-left: 8px;
    margin-right: 4px;
    transition: color 0.3s;
}

.logout-icon:hover {
    color: #f89898;
}

.logout-icon.disabled {
    color: #c0c4cc;
    cursor: not-allowed;
}

.logout-icon.disabled:hover {
    color: #c0c4cc;
}

.template-list {
    margin-left: 20px;
    margin-top: 10px;
}

.template-item {
    display: flex;
    align-items: center;
    padding: 8px 12px;
    background: #fff;
    border-radius: 4px;
    margin-bottom: 5px;
    cursor: pointer;
    transition: all 0.3s;
}

.template-item:hover {
    background: #f0f9ff;
}

.template-item.active {
    background: #ecf5ff;
    border-left: 3px solid #409eff;
}

.template-main {
    flex: 1;
}

.template-name {
    font-weight: 500;
    color: #303133;
    display: flex;
    align-items: center;
    gap: 6px;
}

.unsaved-indicator {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background-color: #f56c6c;
    flex-shrink: 0;
    animation: pulse-red 2s infinite;
}

@keyframes pulse-red {
    0% {
        opacity: 1;
        transform: scale(1);
    }
    50% {
        opacity: 0.7;
        transform: scale(1.1);
    }
    100% {
        opacity: 1;
        transform: scale(1);
    }
}

.template-desc {
    font-size: 12px;
    color: #909399;
    margin-top: 2px;
}

.main-content {
    padding: 0;
    overflow: hidden;
}

.content-wrapper {
    height: 100%;
    padding: 20px;
    overflow-y: auto;
    scrollbar-width: thin;
    scrollbar-color: #c1c1c1 transparent;
}

.content-wrapper::-webkit-scrollbar {
    width: 6px;
}

.content-wrapper::-webkit-scrollbar-track {
    background: transparent;
}

.content-wrapper::-webkit-scrollbar-thumb {
    background-color: #c1c1c1;
    border-radius: 3px;
}

.content-wrapper::-webkit-scrollbar-thumb:hover {
    background-color: #a8a8a8;
}

.no-selection,
.no-template {
    height: 100%;
    display: flex;
    align-items: center;
    justify-content: center;
}

.form-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 20px;
    padding-bottom: 15px;
    border-bottom: 1px solid #e4e7ed;
}

.form-header h3 {
    margin: 0;
    color: #303133;
}

.template-name-container {
    flex: 1;
    margin-right: 20px;
}

.template-name-display {
    margin: 0;
    color: #303133;
    cursor: pointer;
    padding: 8px 12px;
    border-radius: 4px;
    transition: all 0.3s;
    display: inline-flex;
    align-items: center;
    gap: 8px;
    position: relative;
    max-width: fit-content;
}

.template-name-display:hover {
    background: #f0f9ff;
    color: #409eff;
}

.template-name-display .edit-hint-icon {
    opacity: 0;
    font-size: 14px;
    transition: opacity 0.3s;
}

.template-name-display:hover .edit-hint-icon {
    opacity: 1;
}

.template-name-input {
    max-width: 300px;
}

.header-actions {
    display: flex;
    gap: 10px;
}

.form-section {
    margin-bottom: 20px;
}

.form-section.drag-target {
    border: 2px dashed #409eff;
    background: rgba(64, 158, 255, 0.05);
    transition: all 0.3s ease;
}

.form-section.drag-target .el-card__header {
    background: rgba(64, 158, 255, 0.1);
}

/* 卡片折叠样式 */
.card-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    cursor: pointer;
    user-select: none;
    transition: all 0.3s ease;
}

.card-header:hover {
    color: #409eff;
}

.collapse-icon {
    transition: transform 0.3s ease;
    color: #909399;
}

.collapse-icon:hover {
    color: #409eff;
}

.collapse-icon.collapsed {
    transform: rotate(-90deg);
}

.form-section.collapsed {
    margin-bottom: 10px;
}

.card-content {
    padding-top: 0;
}

.drag-hint {
    float: right;
    color: #409eff;
    font-size: 12px;
    font-weight: 500;
}

.tag-input {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
    align-items: center;
}

.tag-item {
    margin: 0;
}

.tag-input-field {
    width: 100px;
}

.add-tag-btn {
    height: 24px;
}

.cover-uploader {
    position: relative;
    display: inline-block;
    z-index: 1; /* 确保容器有基础层级 */
}

.cover-uploader .cover-image {
    width: 100px;
    height: 60px;
    object-fit: cover;
    border-radius: 4px;
    transition:
        transform 0.3s ease,
        box-shadow 0.3s ease;
    cursor: pointer;
    position: relative; /* 重要：让 z-index 生效 */
}

.cover-uploader .cover-image:hover {
    transform: scale(3) translateX(25px);
    box-shadow: 0 8px 16px rgba(0, 0, 0, 0.2);
    z-index: 999; /* 确保悬浮时在最顶层 */
    position: relative; /* 确保定位生效 */
}

.cover-uploader-icon {
    width: 100px;
    height: 60px;
    border: 1px dashed #d9d9d9;
    border-radius: 4px;
    display: flex;
    align-items: center;
    justify-content: center;
    color: #8c939d;
    font-size: 24px;
}

.upload-tip {
    color: #909399;
    font-size: 12px;
    margin-top: 5px;
}

.video-buttons-group {
    display: flex;
    gap: 10px;
    align-items: center;
    margin-bottom: 10px;
}

.drag-active-tip {
    color: #409eff !important;
    font-weight: 500;
    animation: pulse 1.5s infinite;
}

@keyframes pulse {
    0% {
        opacity: 1;
    }
    50% {
        opacity: 0.7;
    }
    100% {
        opacity: 1;
    }
}

.user-option {
    display: flex;
    align-items: center;
    gap: 8px;
}

.user-option-name {
    font-size: 14px;
}

.empty-users {
    text-align: center;
    margin-top: 50px;
}

/* 登录对话框样式 */
.login-dialog :deep(.el-dialog) {
    margin: 0;
    padding: 0;
    border-radius: 0;
    background: transparent;
    box-shadow: none;
    max-height: 90vh;
}

.login-dialog :deep(.el-dialog__header) {
    display: none;
}

.login-dialog :deep(.el-dialog__body) {
    padding: 0;
    max-height: 90vh;
    overflow: hidden;
}

.login-dialog-content {
    max-height: 90vh;
    overflow-y: auto;
    scrollbar-width: thin;
    scrollbar-color: #c1c1c1 transparent;
}

.login-dialog-content::-webkit-scrollbar {
    width: 6px;
}

.login-dialog-content::-webkit-scrollbar-track {
    background: transparent;
}

.login-dialog-content::-webkit-scrollbar-thumb {
    background-color: #c1c1c1;
    border-radius: 3px;
}

.login-dialog-content::-webkit-scrollbar-thumb:hover {
    background-color: #a8a8a8;
}

.login-dialog-content .login-view {
    min-height: auto;
    padding: 0;
    background: transparent;
}

.login-dialog-content .login-container {
    max-width: none;
    width: 100%;
    padding: 0;
}

.login-dialog-content .login-card {
    margin: 0;
    box-shadow: 0 10px 40px rgba(0, 0, 0, 0.2);
    border-radius: 16px;
}

.checkbox-group {
    display: flex;
    flex-direction: column;
    gap: 12px;
}

/* 表单提示样式 */
.form-tip {
    font-size: 12px;
    color: #909399;
    margin-top: 5px;
    line-height: 1.4;
}

.form-tip div {
    margin-bottom: 2px;
}

/* 分区选择器样式 */
.category-trigger {
    width: 100%;
    display: flex !important;
    justify-content: space-between !important;
    align-items: center !important;
    border: 1px solid #dcdfe6;
    background: #fff;
    color: #606266;
    padding: 8px 15px;
    border-radius: 4px;
    cursor: pointer;
    transition: border-color 0.3s;
    position: relative;
}

.category-trigger .category-text {
    flex: 1;
    text-align: left;
    padding-right: 30px; /* 为右侧箭头留出空间 */
}

.category-trigger:hover {
    border-color: #409eff;
}

.category-trigger.el-button--primary {
    background: #fff;
    border-color: #409eff;
    color: #409eff;
}

.category-trigger .placeholder {
    color: #c0c4cc;
}

.category-trigger .arrow-icon {
    position: absolute;
    right: 12px;
    top: 50%;
    transform: translateY(-50%);
    transition: transform 0.3s;
    flex-shrink: 0;
}

/* 分区选择面板 */
.category-selector-panel {
    display: flex;
    height: 360px;
    border-radius: 6px;
    overflow: hidden;
    box-shadow: 0 4px 12px rgba(0, 0, 0, 0.15);
}

.category-list {
    width: 180px;
    background: #f8f9fa;
    border-right: 1px solid #e9ecef;
    overflow-y: auto;
}

.subcategory-list {
    flex: 1;
    background: #fff;
    overflow-y: auto;
}

.category-item {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 12px 16px;
    cursor: pointer;
    transition: all 0.3s;
    border-bottom: 1px solid #f0f2f5;
}

.category-item:hover {
    background: #e6f7ff;
    color: #1890ff;
}

.category-item.active {
    background: #1890ff;
    color: #fff;
}

.category-item.active .arrow-right {
    color: #fff;
}

.category-name {
    font-size: 13px;
}

.arrow-right {
    color: #c0c4cc;
    font-size: 12px;
    transition: color 0.3s;
}

.subcategory-item {
    padding: 12px 16px;
    cursor: pointer;
    transition: all 0.3s;
    border-bottom: 1px solid #f0f2f5;
}

.subcategory-item:hover {
    background: #f0f9ff;
}

.subcategory-item.active {
    background: #e6f7ff;
    border-left: 3px solid #1890ff;
}

.subcategory-content {
    display: flex;
    flex-direction: column;
    gap: 4px;
}

.subcategory-name {
    font-size: 14px;
    font-weight: 500;
    color: #303133;
}

.subcategory-desc {
    font-size: 12px;
    color: #909399;
    line-height: 1.4;
}

.empty-subcategory {
    height: 100%;
    display: flex;
    align-items: center;
    justify-content: center;
}

/* 滚动条样式 */
.category-list::-webkit-scrollbar,
.subcategory-list::-webkit-scrollbar {
    width: 6px;
}

.category-list::-webkit-scrollbar-track,
.subcategory-list::-webkit-scrollbar-track {
    background: transparent;
}

.category-list::-webkit-scrollbar-thumb,
.subcategory-list::-webkit-scrollbar-thumb {
    background-color: #c1c1c1;
    border-radius: 3px;
}

.category-list::-webkit-scrollbar-thumb:hover,
.subcategory-list::-webkit-scrollbar-thumb:hover {
    background-color: #a8a8a8;
}

/* 上传操作区域 */
.upload-actions {
    display: flex;
    justify-content: center;
    align-items: center;
    gap: 16px;
    padding: 20px 0;
    margin-top: 20px;
    border-top: 1px solid #e4e7ed;
}

.upload-actions .el-button {
    min-width: 140px;
}

/* 已上传文件列表样式 */
.uploaded-videos-section {
    --video-item-height: 35px; /* 定义CSS变量 */
    margin-bottom: 20px;
    padding-bottom: 20px;
    border-bottom: 1px solid #f0f2f5;
}

.uploaded-videos-section h4 {
    margin: 0 0 16px 0;
    font-size: 14px;
    font-weight: 500;
    color: #303133;
}

.uploaded-videos-list {
    display: flex;
    flex-direction: column;
    gap: 4px;
    max-height: 250px;
    overflow-y: auto;
    scrollbar-width: thin;
    scrollbar-color: #c1c1c1 transparent;
    border-radius: 6px;
    border: 1px solid #e9ecef;
    padding: 8px;
    background: #fafbfc;
}

.uploaded-videos-list::-webkit-scrollbar {
    width: 6px;
}

.uploaded-videos-list::-webkit-scrollbar-track {
    background: transparent;
}

.uploaded-videos-list::-webkit-scrollbar-thumb {
    background-color: #c1c1c1;
    border-radius: 3px;
}

.uploaded-videos-list::-webkit-scrollbar-thumb:hover {
    background-color: #a8a8a8;
}

.uploaded-video-item {
    display: flex;
    align-items: center;
    padding: 4px 8px;
    background: #fff;
    border-radius: 4px;
    border: 1px solid #e9ecef;
    transition: all 0.3s;
    min-height: var(--video-item-height);
    flex-shrink: 0;
}

.uploaded-video-item:hover {
    background: #f0f9ff;
    border-color: #b3d8ff;
}

.uploaded-video-item:hover .video-actions {
    opacity: 1;
}

.video-status-icon {
    margin-right: 6px;
    display: flex;
    align-items: center;
    justify-content: center;
    width: 16px;
    height: 16px;
}

.status-complete {
    color: #67c23a;
    font-size: 14px;
}

.status-uploading {
    color: #409eff;
    font-size: 12px;
    animation: rotate 1s linear infinite;
}

.status-pending {
    color: #909399;
    font-size: 12px;
}

@keyframes rotate {
    from {
        transform: rotate(0deg);
    }
    to {
        transform: rotate(360deg);
    }
}

.video-info {
    flex: 1;
    display: flex;
    flex-direction: column;
    gap: 1px;
}

.video-title-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
}

.video-title-container {
    flex: 1;
    min-width: 0;
}

.video-title {
    font-size: 12px;
    font-weight: 500;
    color: #303133;
    line-height: 1.2;
    cursor: pointer;
    display: flex;
    align-items: center;
    gap: 3px;
    padding: 1px 3px;
    border-radius: 2px;
    transition: all 0.3s;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
}

.video-title:hover {
    background: #ecf5ff;
    color: #409eff;
}

.video-title:hover .edit-icon {
    opacity: 1;
}

.edit-icon {
    opacity: 0;
    font-size: 10px;
    transition: opacity 0.3s;
}

.video-status {
    flex-shrink: 0;
}

.video-status .status-text {
    padding: 1px 4px;
    border-radius: 2px;
    font-size: 9px;
    font-weight: 500;
    line-height: 1.2;
}

.video-status .status-text.complete {
    background: #f0f9ff;
    color: #67c23a;
}

.video-status .status-text.uploading {
    background: #ecf5ff;
    color: #409eff;
}

.video-status .status-text.pending {
    background: #f4f4f5;
    color: #909399;
}

.video-title-edit {
    width: 150px;
}

.video-actions {
    margin-left: 6px;
    opacity: 0;
    transition: opacity 0.3s;
    display: flex;
    gap: 2px;
}

.status-text.pending {
    background: #f4f4f5;
    color: #909399;
}

.progress-section {
    display: flex;
    flex-direction: column;
    gap: 1px;
    margin-top: 1px;
}

.progress-bar-container {
    display: flex;
    align-items: center;
    gap: 4px;
}

.progress-bar-container .el-progress {
    flex: 1;
    min-width: 60px;
}

.progress-text {
    font-size: 9px;
    font-weight: 500;
    color: #606266;
    min-width: 25px;
    text-align: right;
}

/* 拖拽覆盖层样式 */
.drag-overlay {
    position: fixed;
    top: 0;
    left: 0;
    right: 0;
    bottom: 0;
    background: rgba(64, 158, 255, 0.9);
    z-index: 9999;
    display: flex;
    align-items: center;
    justify-content: center;
    backdrop-filter: blur(4px);
}

.drag-content {
    text-align: center;
    color: white;
    padding: 40px;
    border-radius: 12px;
    background: rgba(255, 255, 255, 0.1);
    border: 2px dashed rgba(255, 255, 255, 0.8);
    max-width: 500px;
}

.drag-icon {
    font-size: 64px;
    margin-bottom: 20px;
    animation: bounce 2s infinite;
}

@keyframes bounce {
    0%,
    20%,
    50%,
    80%,
    100% {
        transform: translateY(0);
    }
    40% {
        transform: translateY(-10px);
    }
    60% {
        transform: translateY(-5px);
    }
}

.drag-content h3 {
    margin: 0 0 10px 0;
    font-size: 24px;
    font-weight: 600;
}

.drag-content p {
    margin: 8px 0;
    font-size: 16px;
    opacity: 0.9;
}

.drag-content .warning-text {
    color: #ffd700;
    font-weight: 500;
    margin-top: 15px;
}

.upload-speed {
    font-size: 9px;
    color: #909399;
    text-align: right;
    font-family: 'Courier New', monospace;
    line-height: 1.2;
}
</style>

<style>
/* 全局样式：分区选择器popover */
.category-popover {
    padding: 0 !important;
}

.category-popover .el-popover__arrow {
    display: none;
}
</style>
