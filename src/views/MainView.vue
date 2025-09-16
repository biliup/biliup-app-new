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
                    <div class="app-version">(v{{ currentVer }})</div>
                </div>
                <div class="header-center">
                    <el-button type="info" size="small" @click="exportLogs" title="导出日志">
                        导出日志
                    </el-button>
                    <el-button type="primary" size="small" @click="checkUpdate" title="检查更新">
                        检查更新
                    </el-button>
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

                    <!-- 用户列表下拉框 -->
                    <UserList
                        @show-login="showLoginDialog = true"
                        @user-logout="handleLogoutUser"
                    />
                </div>
            </div>
        </el-header>

        <el-container class="main-container">
            <!-- 用户模板侧边栏 -->
            <el-aside width="300px" class="sidebar">
                <div class="sidebar-header">
                    <h3></h3>
                    <div class="header-buttons">
                        <el-checkbox
                            v-model="highlightAutoSubmitting"
                            size="small"
                            class="highlight-checkbox"
                        >
                            <span class="highlight-checkbox-text"> 高亮显示<br />自动提交 </span>
                        </el-checkbox>
                        <el-button
                            type="success"
                            size="small"
                            @click="showLoginDialog = true"
                            :disabled="templateLoading"
                        >
                            <el-icon><user /></el-icon>
                            登录用户
                        </el-button>
                        <el-button
                            type="primary"
                            size="small"
                            @click="showNewTemplateDialog = true"
                            :disabled="!loginUsers.length || templateLoading"
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
                                @click="handleUserExpansion(userTemplate.user.uid)"
                                :class="{ disabled: templateLoading }"
                            >
                                <el-avatar
                                    :src="`data:image/jpeg;base64,${userTemplate.user.avatar}`"
                                    :size="24"
                                    class="user-avatar"
                                >
                                    {{ userTemplate.user.username.charAt(0) }}
                                </el-avatar>
                                <span class="user-name">{{ userTemplate.user.username }}</span>
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
                                            currentTemplateName === template.name,
                                        'auto-submitting':
                                            highlightAutoSubmitting &&
                                            autoSubmittingRecord[
                                                getTemplateKey(userTemplate.user.uid, template.name)
                                            ],
                                        'auto-submitting-simple':
                                            !highlightAutoSubmitting &&
                                            autoSubmittingRecord[
                                                getTemplateKey(userTemplate.user.uid, template.name)
                                            ],
                                        'template-loading':
                                            templateLoading &&
                                            selectedUser?.uid === userTemplate.user.uid &&
                                            currentTemplateName === template.name,
                                        disabled: templateLoading
                                    }"
                                    @click="
                                        handleTemplateSelection(userTemplate.user, template.name)
                                    "
                                >
                                    <div class="template-main">
                                        <div class="template-name">
                                            {{ template.name }}
                                            <span
                                                v-show="
                                                    checkTemplateHasUnsavedChanges(
                                                        userTemplate.user.uid,
                                                        template.name
                                                    )
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
                                        :disabled="templateLoading"
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
            <el-main class="main-content" v-if="currentForm">
                <div class="content-wrapper" ref="contentWrapperRef">
                    <div v-if="!selectedUser" class="no-selection">
                        <el-empty description="请选择用户和模板开始使用" />
                    </div>

                    <div v-else-if="!currentTemplateName" class="no-template">
                        <el-empty description="请选择模板或创建新模板">
                            <el-button type="primary" @click="showNewTemplateDialog = true">
                                新建模板
                            </el-button>
                        </el-empty>
                    </div>

                    <div v-else class="upload-form-container">
                        <div class="form-header">
                            <div class="template-name-container">
                                <h3 class="edit-bv-template-disaplay" v-if="currentTemplate?.aid">
                                    编辑稿件：
                                </h3>
                                <el-tooltip
                                    v-if="currentTemplate?.aid"
                                    content="刷新稿件数据"
                                    placement="top"
                                >
                                    <el-icon
                                        class="refresh-btn"
                                        @click.stop="
                                            reloadTemplateFromAV(
                                                selectedUser.uid,
                                                currentTemplate.aid
                                            )
                                        "
                                    >
                                        <refresh />
                                    </el-icon>
                                </el-tooltip>
                                <h3
                                    v-if="!isEditingTemplateName"
                                    @click="handleTemplateNameEdit"
                                    class="template-name-display"
                                    :class="{ disabled: templateLoading }"
                                    :title="
                                        templateLoading
                                            ? '模板加载中，无法编辑'
                                            : '点击编辑模板名称'
                                    "
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
                                    :disabled="templateLoading"
                                />
                            </div>
                            <div class="header-actions">
                                <el-button @click="resetTemplate" :disabled="templateLoading"
                                    >放弃更改</el-button
                                >
                                <el-button
                                    type="primary"
                                    @click="saveTemplate"
                                    :disabled="templateLoading"
                                    >保存</el-button
                                >
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
                                    :disabled="templateLoading"
                                    >删除</el-button
                                >
                            </div>
                        </div>

                        <el-form :model="currentForm" label-width="80px" class="upload-form">
                            <!-- 基本信息 -->
                            <el-card
                                class="form-section"
                                :class="{ collapsed: cardCollapsed.basic }"
                            >
                                <template #header>
                                    <div class="card-header" @click="toggleCardCollapsed('basic')">
                                        <span>基本信息</span>
                                        <div class="header-actions">
                                            <el-button
                                                type="danger"
                                                text
                                                size="small"
                                                @click.stop="clearCardContent('basic')"
                                                title="清空基本信息"
                                                :disabled="templateLoading"
                                            >
                                                <el-icon><delete /></el-icon>
                                            </el-button>
                                            <el-icon
                                                class="collapse-icon"
                                                :class="{ collapsed: cardCollapsed.basic }"
                                            >
                                                <arrow-down />
                                            </el-icon>
                                        </div>
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
                                                :disabled="templateLoading"
                                            />
                                        </el-form-item>

                                        <el-form-item label="封面">
                                            <div
                                                class="cover-uploader"
                                                action="#"
                                                @click="handleCoverSelection"
                                                v-loading="coverLoading"
                                                :class="{ disabled: templateLoading }"
                                            >
                                                <img
                                                    v-if="coverDisplayUrl && !coverLoading"
                                                    :src="coverDisplayUrl"
                                                    class="cover-image"
                                                />
                                                <el-icon
                                                    v-else-if="!coverLoading"
                                                    class="cover-uploader-icon"
                                                >
                                                    <plus />
                                                </el-icon>
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
                                                        :disabled="templateLoading"
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
                                            <el-radio-group
                                                v-model="currentForm.copyright"
                                                :disabled="templateLoading"
                                            >
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
                                                :disabled="templateLoading"
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
                                    <div class="card-header">
                                        <div
                                            style="
                                                display: flex;
                                                align-items: center;
                                                gap: 12px;
                                                flex: 1;
                                            "
                                            @click="toggleCardCollapsed('videos')"
                                        >
                                            <span style="cursor: pointer">视频文件</span>
                                            <el-button
                                                type="success"
                                                size="small"
                                                @click.stop="checkVideoStatus"
                                                v-if="
                                                    currentForm.videos &&
                                                    currentForm.videos.length > 0 &&
                                                    currentTemplate?.aid
                                                "
                                                :disabled="templateLoading"
                                            >
                                                视频转码状态
                                            </el-button>
                                        </div>
                                        <div class="header-actions">
                                            <span v-if="isDragOver" class="drag-hint"
                                                >拖拽文件到此处添加</span
                                            >
                                            <el-icon
                                                class="collapse-icon"
                                                :class="{ collapsed: cardCollapsed.videos }"
                                                @click="toggleCardCollapsed('videos')"
                                            >
                                                <arrow-down />
                                            </el-icon>
                                        </div>
                                    </div>
                                </template>

                                <el-collapse-transition>
                                    <div v-show="!cardCollapsed.videos" class="card-content">
                                        <VideoList
                                            v-model:videos="videos"
                                            :is-drag-over="isDragOver"
                                            :uploading="uploading"
                                            :template-title="currentTemplateName"
                                            :disabled="templateLoading"
                                            @select-video="selectVideoWithTauri"
                                            @clear-all-videos="clearAllVideos"
                                            @remove-file="removeUploadedFile"
                                            @create-upload="createUpload"
                                            @add-videos-to-form="handleAddVideosToForm"
                                            @submit-template="handleSubmitTemplate"
                                        />
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
                                        <div class="header-actions">
                                            <el-button
                                                type="danger"
                                                text
                                                size="small"
                                                @click.stop="clearCardContent('tags')"
                                                title="清空标签设置"
                                                :disabled="templateLoading"
                                            >
                                                <el-icon><delete /></el-icon>
                                            </el-button>
                                            <el-icon
                                                class="collapse-icon"
                                                :class="{ collapsed: cardCollapsed.tags }"
                                            >
                                                <arrow-down />
                                            </el-icon>
                                        </div>
                                    </div>
                                </template>

                                <el-collapse-transition>
                                    <div v-show="!cardCollapsed.tags" class="card-content">
                                        <el-form-item label="视频标签">
                                            <TagView
                                                ref="tagViewRef"
                                                v-model="tags"
                                                :disabled="templateLoading"
                                            />
                                        </el-form-item>

                                        <el-form-item v-if="!currentForm.aid" label="参与活动">
                                            <TopicView
                                                v-model="currentForm.mission_id"
                                                v-model:topic-id="currentForm.topic_id"
                                                :user-uid="selectedUser?.uid"
                                                mode="selector"
                                                :disabled="templateLoading"
                                            />
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
                                        <div class="header-actions">
                                            <el-button
                                                type="danger"
                                                text
                                                size="small"
                                                @click.stop="clearCardContent('description')"
                                                title="清空视频描述"
                                                :disabled="templateLoading"
                                            >
                                                <el-icon><delete /></el-icon>
                                            </el-button>
                                            <el-icon
                                                class="collapse-icon"
                                                :class="{ collapsed: cardCollapsed.description }"
                                            >
                                                <arrow-down />
                                            </el-icon>
                                        </div>
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
                                                :disabled="templateLoading"
                                            />
                                        </el-form-item>

                                        <el-form-item label="粉丝动态">
                                            <el-input
                                                v-model="currentForm.dynamic"
                                                placeholder="发布时的动态内容"
                                                maxlength="233"
                                                show-word-limit
                                                :disabled="templateLoading"
                                            />
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
                                        <div class="header-actions">
                                            <el-button
                                                type="danger"
                                                text
                                                size="small"
                                                @click.stop="clearCardContent('advanced')"
                                                title="清空高级选项"
                                                :disabled="templateLoading"
                                            >
                                                <el-icon><delete /></el-icon>
                                            </el-button>
                                            <el-icon
                                                class="collapse-icon"
                                                :class="{ collapsed: cardCollapsed.advanced }"
                                            >
                                                <arrow-down />
                                            </el-icon>
                                        </div>
                                    </div>
                                </template>

                                <el-collapse-transition>
                                    <div v-show="!cardCollapsed.advanced" class="card-content">
                                        <el-form-item label="开启水印">
                                            <div div class="checkbox-group">
                                                <el-checkbox
                                                    v-model="currentForm.watermark"
                                                    :true-value="1"
                                                    :false-value="0"
                                                    :disabled="templateLoading"
                                                >
                                                    开启 (本功能只对本次上传的视频生效)
                                                </el-checkbox>
                                            </div>
                                        </el-form-item>
                                        <el-form-item v-if="!currentForm.aid" label="定时发布">
                                            <el-date-picker
                                                v-model="dtimeDate"
                                                type="datetime"
                                                placeholder="选择发布时间"
                                                format="YYYY-MM-DD HH:mm:ss"
                                                :disabled="templateLoading"
                                                :disabled-date="
                                                    (date: Date) => {
                                                        const now = new Date()
                                                        const twoHoursLater = new Date(
                                                            now.getTime() + 2 * 60 * 60 * 1000
                                                        )
                                                        const fifteenDaysLater = new Date(
                                                            now.getTime() + 15 * 24 * 60 * 60 * 1000
                                                        )

                                                        return (
                                                            date < twoHoursLater ||
                                                            date > fifteenDaysLater
                                                        )
                                                    }
                                                "
                                            />
                                        </el-form-item>

                                        <el-form-item label="字幕设置">
                                            <el-checkbox
                                                v-model="currentForm.open_subtitle"
                                                :disabled="templateLoading"
                                            >
                                                开启字幕功能
                                            </el-checkbox>
                                        </el-form-item>

                                        <el-form-item label="互动功能">
                                            <div div class="checkbox-group">
                                                <el-checkbox
                                                    v-model="currentForm.interactive"
                                                    :true-value="1"
                                                    :false-value="0"
                                                    :disabled="templateLoading"
                                                >
                                                    开启
                                                </el-checkbox>
                                            </div>
                                        </el-form-item>

                                        <el-form-item label="加入合集">
                                            <SeasonView
                                                v-model="currentForm.season_id"
                                                v-model:section-id="currentForm.section_id"
                                                :user-uid="selectedUser?.uid"
                                                :disabled="templateLoading"
                                            />
                                        </el-form-item>

                                        <el-form-item label="音质设置">
                                            <div class="checkbox-group">
                                                <el-checkbox
                                                    v-model="currentForm.dolby"
                                                    :true-value="1"
                                                    :false-value="0"
                                                    :disabled="templateLoading"
                                                >
                                                    杜比音效
                                                </el-checkbox>
                                                <el-checkbox
                                                    v-model="currentForm.lossless_music"
                                                    :true-value="1"
                                                    :false-value="0"
                                                    :disabled="templateLoading"
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
                                                    :disabled="templateLoading"
                                                >
                                                    禁止转载
                                                </el-checkbox>
                                                <el-checkbox
                                                    v-model="currentForm.open_elec"
                                                    :true-value="1"
                                                    :false-value="0"
                                                    :disabled="templateLoading"
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
                                                    :disabled="templateLoading"
                                                >
                                                    UP主精选评论
                                                </el-checkbox>
                                                <el-checkbox
                                                    v-model="currentForm.up_close_reply"
                                                    :true-value="1"
                                                    :false-value="0"
                                                    :disabled="templateLoading"
                                                >
                                                    关闭评论
                                                </el-checkbox>
                                                <el-checkbox
                                                    v-model="currentForm.up_close_danmu"
                                                    :true-value="1"
                                                    :false-value="0"
                                                    :disabled="templateLoading"
                                                >
                                                    关闭弹幕
                                                </el-checkbox>
                                            </div>
                                        </el-form-item>

                                        <el-form-item label="可见性">
                                            <el-checkbox
                                                v-model="currentForm.is_only_self"
                                                :true-value="1"
                                                :false-value="0"
                                                :disabled="templateLoading"
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
                                    type="primary"
                                    size="large"
                                    :loading="submitting"
                                    @click="submitTemplate"
                                    :disabled="
                                        templateLoading ||
                                        !currentForm.videos ||
                                        currentForm.videos.length === 0 ||
                                        !currentForm.title ||
                                        currentForm.title.trim() === ''
                                    "
                                >
                                    <el-icon v-if="!allFilesUploaded && !submitting"
                                        ><loading
                                    /></el-icon>
                                    <el-icon v-else-if="!submitting"><check /></el-icon>
                                    {{
                                        !getCurrentAutoSubmitting
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

        <!-- 新建模板组件 -->
        <NewTemplete
            ref="newTemplateRef"
            v-model="showNewTemplateDialog"
            @template-created="handleTemplateCreated"
        />

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
            <div class="login-dialog-content" @click.stop>
                <LoginView
                    @login-success="handleLoginSuccess"
                    @loading-change="loginLoading = $event"
                />
            </div>
        </el-dialog>

        <!-- 用户配置对话框 -->
        <UserConfig v-model="userConfigVisible" :user="configUser" />

        <!-- 视频状态对话框 -->
        <VideoStatus
            v-model="showVideoStatusDialog"
            :videos="currentForm?.videos || []"
            :user="selectedUser"
            :template-aid="currentTemplate?.aid"
            @reload-template="
                () =>
                    selectedUser &&
                    currentTemplate?.aid &&
                    reloadTemplateFromAV(selectedUser.uid, currentTemplate.aid)
            "
        />

        <!-- 全局配置对话框 -->
        <GlobalConfigView v-model="showGlobalConfigDialog" />
    </div>
</template>

<script setup lang="ts">
import { ref, onMounted, computed, nextTick, watch, onUnmounted } from 'vue'
import { v4 as uuidv4 } from 'uuid'
import { useAuthStore } from '../stores/auth'
import { useUserConfigStore, TemplateConfig } from '../stores/user_config'
import { useUtilsStore } from '../stores/utils'
import { useUploadStore } from '../stores/upload'
import { ElMessageBox } from 'element-plus'
import {
    ArrowDown,
    Plus,
    MoreFilled,
    UploadFilled,
    User,
    Check,
    Edit,
    Setting,
    Refresh,
    Delete
} from '@element-plus/icons-vue'
import { open, save } from '@tauri-apps/plugin-dialog'
import { copyFile, remove } from '@tauri-apps/plugin-fs'
import { openUrl } from '@tauri-apps/plugin-opener'
import { listen } from '@tauri-apps/api/event'
import LoginView from '../components/LoginView.vue'
import UserConfig from '../components/UserConfig.vue'
import TopicView from '../components/TopicView.vue'
import SeasonView from '../components/SeasonView.vue'
import UploadQueue from '../components/UploadQueue.vue'
import GlobalConfigView from '../components/GlobalConfig.vue'
import NewTemplete from '../components/NewTemplete.vue'
import VideoList from '../components/VideoList.vue'
import UserList from '../components/UserList.vue'
import VideoStatus from '../components/VideoStatus.vue'
import TagView from '../components/TagView.vue'

const authStore = useAuthStore()
const userConfigStore = useUserConfigStore()
const uploadStore = useUploadStore()
const utilsStore = useUtilsStore()

// 计算属性
const loginUsers = computed(() => authStore.loginUsers)
const userTemplates = computed(() => userConfigStore.userTemplates)
const typeList = computed(() => utilsStore.typelist)

const currentVer = ref<string>('')

// 封面显示URL
const coverDisplayUrl = ref<string>('')
const coverLoading = ref<boolean>(false)

// 响应式数据
const selectedUser = ref<any>(null)
const currentTemplateName = ref<string>('')
const showNewTemplateDialog = ref(false)
const showLoginDialog = ref(false)
const showGlobalConfigDialog = ref(false)
const loginLoading = ref(false)
const uploading = ref(false)
const submitting = ref(false)
const templateLoading = ref(false) // 模板加载状态锁

// 视频状态对话框
const showVideoStatusDialog = ref(false)

// 组件引用
const newTemplateRef = ref<InstanceType<typeof NewTemplete> | null>(null)
const tagViewRef = ref<InstanceType<typeof TagView> | null>(null)
// 自动提交状态记录 - 记录每个模板的自动提交状态
const autoSubmittingRecord = ref<Record<string, boolean>>({})
// 全局自动提交检查间隔
let autoSubmitInterval: number | null = null

// 高亮显示自动提交状态的开关
const highlightAutoSubmitting = ref<boolean>(
    localStorage.getItem('highlightAutoSubmitting') === 'true'
)

// 监听高亮开关变化，保存到localStorage
watch(highlightAutoSubmitting, newValue => {
    localStorage.setItem('highlightAutoSubmitting', String(newValue))
})

// 生成模板键名
const getTemplateKey = (uid: number, templateName: string) => `${uid}-${templateName}`

// 获取当前模板的自动提交状态
const getCurrentAutoSubmitting = computed(() => {
    if (!selectedUser.value || !currentTemplateName.value) return false
    const key = getTemplateKey(selectedUser.value.uid, currentTemplateName.value)
    return autoSubmittingRecord.value[key] || false
})

// 设置模板的自动提交状态
const setAutoSubmitting = (uid: number, templateName: string, status: boolean) => {
    const key = getTemplateKey(uid, templateName)
    if (status) {
        autoSubmittingRecord.value[key] = true
    } else {
        delete autoSubmittingRecord.value[key]
    }
}

// 检查是否有任何模板在自动提交
const hasAnyAutoSubmitting = computed(() => {
    return Object.keys(autoSubmittingRecord.value).length > 0
})

// 全局自动提交检查函数
const checkAutoSubmitAll = async () => {
    const templateKeys = Object.keys(autoSubmittingRecord.value)

    for (const templateKey of templateKeys) {
        const [uidStr, templateName] = templateKey.split('-', 2)
        const uid = parseInt(uidStr)

        if (isNaN(uid) || !templateName) continue

        // 获取用户和模板配置
        const user = loginUsers.value.find(u => u.uid === uid)
        if (!user || !userConfigStore.configRoot?.config[uid]?.templates[templateName]) {
            // 如果用户或模板不存在，清除自动提交状态
            setAutoSubmitting(uid, templateName, false)
            continue
        }

        const template = userConfigStore.configRoot.config[uid].templates[templateName]

        // 检查是否所有文件都已上传完成
        if (template.videos && template.videos.length > 0) {
            const allUploaded = template.videos.every(video => video.complete && video.path === '')

            if (allUploaded && autoSubmittingRecord.value[templateKey]) {
                // 文件已全部上传完成，执行提交
                setAutoSubmitting(uid, templateName, false)
                try {
                    await performTemplateSubmit(uid, templateName, template)
                } catch (error) {
                    console.error(`模板 ${templateKey} 自动提交失败:`, error)
                }
            }
        } else {
            // 没有视频文件，清除自动提交状态
            setAutoSubmitting(uid, templateName, false)
        }
    }

    // 如果没有模板在自动提交，停止间隔检查
    if (!hasAnyAutoSubmitting.value && autoSubmitInterval) {
        clearInterval(autoSubmitInterval)
        autoSubmitInterval = null
    }
}

// 启动全局自动提交检查
const startAutoSubmitCheck = () => {
    if (!autoSubmitInterval) {
        autoSubmitInterval = setInterval(checkAutoSubmitAll, 1000)
    }
}

// 执行模板提交
const performTemplateSubmit = async (uid: number, templateName: string, template: any) => {
    const user = loginUsers.value.find(u => u.uid === uid)
    if (!user) throw new Error('用户不存在')

    submitting.value = true
    try {
        const resp = (await uploadStore.submitTemplate(uid, template)) as any

        // 更新最后提交时间（只对当前模板）
        if (selectedUser.value?.uid === uid && currentTemplateName.value === templateName) {
            lastSubmit.value = new Date().toLocaleString()
        }

        utilsStore.showMessage(`视频${resp.bvid}提交成功 (模板: ${templateName})`, 'success')
        console.log(`视频${resp.bvid}提交成功 (模板: ${templateName})`, 'success')

        if (resp && resp.aid && utilsStore.hasSeason) {
            try {
                const old_season_id = await utilsStore.getVideoSeason(uid, resp.aid)
                let add = old_season_id && old_season_id !== 0 ? false : true

                if (template && old_season_id !== template.season_id && template.videos[0]?.cid) {
                    const new_season_id = template.season_id || 0
                    const new_section_id = template.section_id || 0
                    await utilsStore.switchSeason(
                        uid,
                        resp.aid,
                        template.videos[0]?.cid,
                        new_season_id,
                        new_section_id,
                        template.title,
                        add
                    )

                    const season_title =
                        utilsStore.seasonlist.find((s: any) => s.season_id === template.season_id)
                            ?.title || template.season_id
                    utilsStore.showMessage(`视频${resp.bvid}加入合集${season_title}`, 'success')
                    console.log(`视频${resp.bvid}加入合集${season_title}`, 'success')
                }
            } catch (error) {
                console.error('设置合集失败: ', error)
                utilsStore.showMessage(`设置合集失败: ${error}`, 'error')
            }
        }

        setTimeout(async () => {
            try {
                if (!template.aid) {
                    const userConfig = userConfigStore.configRoot?.config[uid]
                    if (userConfig && userConfig.auto_edit && newTemplateRef.value) {
                        // 新增稿件且auto_edit开启，创建编辑模板
                        await newTemplateRef.value.createTemplateFromBV(
                            uid,
                            resp.bvid,
                            resp.bvid,
                            true
                        )
                        utilsStore.showMessage('从BV号创建模板成功', 'success')
                    }
                } else {
                    if (
                        selectedUser.value?.uid === uid &&
                        currentTemplateName.value === templateName
                    ) {
                        reloadTemplateFromAV(uid, template.aid)
                    }
                }
            } catch (error) {
                utilsStore.showMessage(`${error}`, 'error')
            } finally {
                submitting.value = false
            }
        }, 500)
    } catch (error) {
        console.error('模板提交失败:', error)
        utilsStore.showMessage(`模板提交失败: ${error}`, 'error')
        submitting.value = false
    }
}
const lastSubmit = ref<string>('')

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

// 内容容器引用
const contentWrapperRef = ref<HTMLElement | null>(null)

// 用户配置相关
const userConfigVisible = ref(false)
const configUser = ref<any>(null)

// 分区数据
const selectedCategory = ref<any>(null)
const selectedSubCategory = ref<any>(null)
const categoryPopoverVisible = ref(false)
let generalUpdateTimer: number | null = null

const currentTemplate = computed(() => {
    if (!selectedUser.value || !currentTemplateName.value || !userConfigStore.configRoot?.config) {
        return null
    }
    const userConfig = userConfigStore.configRoot.config[selectedUser.value.uid]
    if (!userConfig || !userConfig.templates[currentTemplateName.value]) {
        return null
    }
    return userConfig.templates[currentTemplateName.value]
})

// 当前表单数据 - 直接操作模板配置
const currentForm = computed({
    get() {
        return currentTemplate.value
    },
    set(value) {
        if (
            !selectedUser.value ||
            !currentTemplateName.value ||
            !userConfigStore.configRoot?.config ||
            !value
        ) {
            return
        }
        const userConfig = userConfigStore.configRoot.config[selectedUser.value.uid]
        if (userConfig && userConfig.templates[currentTemplateName.value]) {
            userConfig.templates[currentTemplateName.value] = value
        }
    }
})

const tags = ref<string[]>([])

// 日期选择器的计算属性 - 处理时间戳转换
const dtimeDate = computed({
    get() {
        return currentForm.value?.dtime ? new Date(currentForm.value.dtime * 1000) : null
    },
    set(value: Date | null) {
        if (currentForm.value) {
            currentForm.value.dtime = value ? Math.floor(value.getTime() / 1000) : undefined
        }
    }
})

// 视频数组的计算属性 - 确保始终返回数组
const videos = computed({
    get() {
        return currentForm.value?.videos || []
    },
    set(value: any[]) {
        if (currentForm.value) {
            currentForm.value.videos = value
        }
    }
})

// 检查指定模板是否有未保存的改动
const checkTemplateHasUnsavedChanges = (uid: number, templateName: string): boolean => {
    if (!userConfigStore.configRoot?.config || !userConfigStore.configBase?.config) {
        return false
    }

    const currentUserConfig = userConfigStore.configRoot.config[uid]
    const baseUserConfig = userConfigStore.configBase.config[uid]

    if (!currentUserConfig?.templates[templateName] || !baseUserConfig?.templates[templateName]) {
        return false
    }

    const currentTemplateData = currentUserConfig.templates[templateName]
    const baseTemplateData = baseUserConfig.templates[templateName]

    return hasUnsavedChanges(baseTemplateData, currentTemplateData)
}

// 生命周期
// 监听封面变化，异步加载显示用的封面URL
watch(
    () => currentForm.value?.cover,
    async (newCover: string | undefined) => {
        if (newCover && selectedUser.value) {
            try {
                coverLoading.value = true
                const downloadedCover = await utilsStore.downloadCover(
                    selectedUser.value.uid,
                    newCover
                )
                coverDisplayUrl.value = downloadedCover || ''
            } catch (error) {
                console.error('Failed to download cover:', error)
                coverDisplayUrl.value = ''
            } finally {
                coverLoading.value = false
            }
        } else {
            coverDisplayUrl.value = ''
            coverLoading.value = false
        }
    }
)

// 监听标签变化，更新表单数据
watch(
    () => tags.value,
    (newTags: string[]) => {
        if (currentForm.value) {
            currentForm.value.tag = newTags.join(',')
        }
    },
    { deep: true }
)

// 监听表单标签变化，更新标签数组
watch(
    () => currentForm.value?.tag,
    (newTag: string | undefined) => {
        const newTags = newTag ? newTag.split(',').filter(tag => tag.trim()) : []
        if (JSON.stringify(newTags) !== JSON.stringify(tags.value)) {
            tags.value = newTags
        }
    }
)

// 监听表单分区变化，更新分区选择（双向绑定）
watch(
    () => currentForm.value?.tid,
    (newTid: number | undefined) => {
        if (newTid && newTid > 0) {
            // 根据tid设置选中的分区
            setSelectedCategoryByTid(newTid)
        } else {
            // 如果没有分区信息，清空分区选择
            selectedCategory.value = null
            selectedSubCategory.value = null
        }
    }
)

// 监听用户切换，重新加载封面
watch(
    () => selectedUser.value,
    async (newUser: any) => {
        if (currentForm.value?.cover && newUser) {
            try {
                coverLoading.value = true
                const downloadedCover = await utilsStore.downloadCover(
                    newUser.uid,
                    currentForm.value.cover
                )
                coverDisplayUrl.value = downloadedCover || ''
            } catch (error) {
                console.error('Failed to download cover:', error)
                coverDisplayUrl.value = ''
            } finally {
                coverLoading.value = false
            }
        } else {
            coverDisplayUrl.value = ''
            coverLoading.value = false
        }
    }
)

let keyboardCleanup: (() => void) | null = null

const forwardConsole = (fnName: keyof Console, logger: (level: string, ...args: any[]) => void) => {
    const original = console[fnName] as (...args: any[]) => void
    ;(console as any)[fnName] = (...args: any[]) => {
        original(...args)
        logger(fnName as string, ...args)
    }
}

onMounted(async () => {
    await initializeData()
    await setupDragAndDrop()
    keyboardCleanup = await setupKeyboardShortcuts()

    forwardConsole('log', utilsStore.log)
    forwardConsole('error', utilsStore.log)
    forwardConsole('warn', utilsStore.log)

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

    // 清理自动提交间隔检查
    if (autoSubmitInterval) {
        clearInterval(autoSubmitInterval)
        autoSubmitInterval = null
    }

    if (generalUpdateTimer) {
        clearInterval(generalUpdateTimer)
        generalUpdateTimer = null
    }

    // 清理所有自动提交状态
    autoSubmittingRecord.value = {}
})

// 初始化数据
const initializeData = async () => {
    try {
        currentVer.value = (await utilsStore.getCurrentVersion()) as string
        // 获取登录用户
        await authStore.getLoginUsers()

        // 构建用户模板列表
        if (loginUsers.value.length > 0) {
            await utilsStore.initTypeList(loginUsers.value[0].uid)
            await utilsStore.initTopicList(loginUsers.value[0].uid)
            await userConfigStore.buildUserTemplates(loginUsers.value)
            await uploadStore.getUploadQueue()
            if (!generalUpdateTimer) {
                generalUpdateTimer = setInterval(() => {
                    if (authStore.loginUsers.length > 0) {
                        uploadStore.getUploadQueue()
                    }
                    for (const task of uploadStore.uploadQueue) {
                        if (task.status === 'Completed') {
                            const templateName = task.template
                            const uid = task.user.uid
                            const videos =
                                userConfigStore.configRoot?.config[uid]?.templates[templateName]
                                    ?.videos || []

                            const video = videos.find(v => v.id === task.video?.id)
                            if (video && video.filename !== task.video?.filename) {
                                video.filename = task.video.filename
                                video.path = task.video.path
                                video.complete = true
                                video.finished_at = task.finished_at
                            }
                        }
                    }
                }, 666) // 更新上传队列
            }
        }

        setTimeout(async () => {
            await restoreTemplateSelection()
            restoreCardCollapsedState()
        }, 100)
    } catch (error) {
        console.error('初始化数据失败: ', error)
        utilsStore.showMessage(`'初始化数据失败: ${error}'`, 'error')
    }
}

const hasUnsavedChanges = (
    baseTemplateData: TemplateConfig,
    currentTemplateData: TemplateConfig
) => {
    // 比较关键字段
    const fieldsToCompare = [
        'title',
        'cover',
        'copyright',
        'source',
        'tid',
        'desc',
        'dynamic',
        'tag',
        'dtime',
        'open_subtitle',
        'interactive',
        'mission_id',
        'topic_id',
        'season_id',
        'section_id',
        'dolby',
        'lossless_music',
        'no_reprint',
        'open_elec',
        'up_selection_reply',
        'up_close_reply',
        'up_close_danmu',
        'is_only_self',
        'watermark'
    ]

    for (const field of fieldsToCompare) {
        const currentValue = (currentTemplateData as any)[field]
        const baseValue = (baseTemplateData as any)[field]

        // 处理 undefined/null/空字符串 的情况
        if (
            (currentValue === undefined || currentValue === null || currentValue === '') &&
            (baseValue === undefined || baseValue === null || baseValue === '')
        ) {
            continue
        }

        if (JSON.stringify(currentValue) !== JSON.stringify(baseValue)) {
            // console.log(field, '有改动')
            // console.log('current: ', JSON.stringify(currentValue), 'vs', JSON.stringify(baseValue))
            return true
        }
    }

    // 特别比较 videos 数组
    const currentVideos = currentTemplateData.videos || []
    const baseVideos = baseTemplateData.videos || []

    if (currentVideos.length !== baseVideos.length) {
        return true
    }

    // 比较视频的关键字段
    for (let i = 0; i < currentVideos.length; i++) {
        const currentVideo = currentVideos[i]
        const baseVideo = baseVideos[i]

        const videoFieldsToCompare = ['title', 'filename', 'desc', 'path']
        for (const field of videoFieldsToCompare) {
            if (
                JSON.stringify((currentVideo as any)[field]) !==
                JSON.stringify((baseVideo as any)[field])
            ) {
                return true
            }
        }
    }

    return false
}

// 设置拖拽功能
const setupDragAndDrop = async () => {
    try {
        // 监听文件拖拽事件
        await listen('tauri://drag-drop', async event => {
            const videos = event.payload as string[]
            isDragOver.value = false
            if (templateLoading.value) {
                utilsStore.showMessage('模板加载中', 'warning')
                return
            }
            await handleDroppedFiles(videos)
        })

        // 监听拖拽悬停事件
        await listen('tauri://drag-over', event => {
            if (!isDragOver.value) console.log('文件拖拽悬停:', event.payload, '，忽略后续日志')
            isDragOver.value = true
        })

        // 监听拖拽取消事件
        await listen('tauri://drag-leave', () => {
            console.log('文件拖拽取消')
            isDragOver.value = false
        })
    } catch (error) {
        console.error('设置拖拽功能失败: ', error)
        utilsStore.showMessage(`'设置拖拽功能失败: ${error}'`, 'error')
    }
}

// 设置键盘快捷键
const setupKeyboardShortcuts = async () => {
    const handleKeydown = (event: KeyboardEvent) => {
        // 禁用 F5 刷新
        if (!event.ctrlKey && event.key === 'F5') {
            event.preventDefault()
            return
        }

        // Ctrl+F5 刷新页面
        if (event.ctrlKey && event.key === 'F5') {
            event.preventDefault()
            window.location.reload()
            return
        }

        if (event.ctrlKey && event.key === 'r') {
            event.preventDefault()
            if (selectedUser.value && currentTemplateName.value) {
                resetTemplate()
            }
            return
        }

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
    // 保存折叠状态到localStorage
    saveCardCollapsedState()
}

// 模板选择记忆功能
const TEMPLATE_SELECTION_KEY = 'last-selected-template'
const CARD_COLLAPSED_KEY = 'card-collapsed-state'

// 是否正在恢复模板选择（避免递归保存）
const isRestoringTemplate = ref(false)

// 保存模板选择到localStorage
const saveTemplateSelection = (userUid: number, templateName: string) => {
    // 如果正在恢复模板，不保存（避免递归）
    if (isRestoringTemplate.value) return

    try {
        const selection = {
            userUid,
            templateName,
            timestamp: Date.now()
        }
        localStorage.setItem(TEMPLATE_SELECTION_KEY, JSON.stringify(selection))
    } catch (error) {
        console.error('保存模板选择失败:', error)
    }
}

// 保存卡片折叠状态
const saveCardCollapsedState = () => {
    try {
        localStorage.setItem(CARD_COLLAPSED_KEY, JSON.stringify(cardCollapsed.value))
    } catch (error) {
        console.error('保存卡片折叠状态失败:', error)
    }
}

// 恢复卡片折叠状态
const restoreCardCollapsedState = () => {
    try {
        const saved = localStorage.getItem(CARD_COLLAPSED_KEY)
        if (saved) {
            const savedState = JSON.parse(saved)
            Object.assign(cardCollapsed.value, savedState)
        }
    } catch (error) {
        console.error('恢复卡片折叠状态失败:', error)
    }
}

// 从localStorage恢复模板选择
const restoreTemplateSelection = async () => {
    try {
        const saved = localStorage.getItem(TEMPLATE_SELECTION_KEY)
        if (!saved) return

        const selection = JSON.parse(saved)
        const { userUid, templateName, timestamp } = selection

        // 检查数据有效性（超过30天的记录自动失效）
        const thirtyDaysAgo = Date.now() - 30 * 24 * 60 * 60 * 1000
        if (timestamp && timestamp < thirtyDaysAgo) {
            localStorage.removeItem(TEMPLATE_SELECTION_KEY)
            return
        }

        // 检查用户是否仍然登录
        const targetUser = loginUsers.value.find(user => user.uid === userUid)
        if (!targetUser) {
            // 用户已不存在，清除记录
            localStorage.removeItem(TEMPLATE_SELECTION_KEY)
            return
        }

        // 检查模板是否仍然存在
        const userTemplate = userTemplates.value.find(ut => ut.user.uid === userUid)
        const template = userTemplate?.templates.find(t => t.name === templateName)
        if (!template) {
            // 模板已不存在，清除记录
            localStorage.removeItem(TEMPLATE_SELECTION_KEY)
            return
        }

        // 确保用户是展开状态
        if (userTemplate && !userTemplate.expanded) {
            toggleUserExpanded(userUid)
        }

        // 设置恢复状态标志
        isRestoringTemplate.value = true

        // 自动选择模板
        await selectTemplate(targetUser, templateName)

        // 恢复完成后重置标志
        isRestoringTemplate.value = false

        console.log(`自动恢复模板选择: ${targetUser.username} - ${templateName}`)
        utilsStore.showMessage(`已恢复上次选择的模板: ${templateName}`, 'success')
    } catch (error) {
        console.error('恢复模板选择失败:', error)
        // 如果恢复失败，清除无效的存储数据
        localStorage.removeItem(TEMPLATE_SELECTION_KEY)
    } finally {
        // 确保标志被重置
        isRestoringTemplate.value = false
    }
}

// 清空卡片内容
const clearCardContent = async (cardType: 'basic' | 'tags' | 'description' | 'advanced') => {
    if (!currentForm.value) {
        utilsStore.showMessage('请先选择模板', 'warning')
        return
    }

    // 如果正在加载模板，禁止清空
    if (templateLoading.value) {
        utilsStore.showMessage('模板正在加载中，请稍后再试', 'warning')
        return
    }

    try {
        // 确认清空
        await ElMessageBox.confirm(
            `确定要清空"${getCardDisplayName(cardType)}"的所有内容吗？`,
            '确认清空',
            {
                confirmButtonText: '确定',
                cancelButtonText: '取消',
                type: 'warning'
            }
        )

        // 根据卡片类型清空相应内容
        switch (cardType) {
            case 'basic':
                currentForm.value.title = ''
                currentForm.value.cover = ''
                currentForm.value.tid = 0
                currentForm.value.copyright = 1
                currentForm.value.source = ''
                // 同步清空分区选择状态
                selectedCategory.value = null
                selectedSubCategory.value = null
                // 清空封面显示
                coverDisplayUrl.value = ''
                break

            case 'tags':
                currentForm.value.tag = ''
                // 同步清空标签数组
                tags.value = []
                // 通过组件引用清空TagView的状态
                tagViewRef.value?.clearTags()
                break

            case 'description':
                currentForm.value.desc = ''
                currentForm.value.dynamic = ''
                break

            case 'advanced':
                currentForm.value.watermark =
                    userConfigStore?.configRoot?.config[selectedUser.value.uid]?.watermark || 0
                currentForm.value.dtime = undefined
                currentForm.value.interactive = 0
                currentForm.value.dolby = 0
                currentForm.value.lossless_music = 0
                currentForm.value.no_reprint = 0
                currentForm.value.open_elec = 0
                currentForm.value.up_selection_reply = 0
                currentForm.value.up_close_reply = 0
                currentForm.value.up_close_danmu = 0
                currentForm.value.atomic_int = 0
                currentForm.value.is_only_self = 0
                break
        }

        utilsStore.showMessage(`已清空"${getCardDisplayName(cardType)}"的内容`, 'success')
    } catch (error) {
        // 用户取消了操作
    }
}

// 获取卡片显示名称
const getCardDisplayName = (cardType: string): string => {
    const cardNames: Record<string, string> = {
        basic: '基本信息',
        tags: '标签设置',
        description: '视频描述',
        videos: '视频文件',
        advanced: '高级选项'
    }
    return cardNames[cardType] || cardType
}

const addVideoToCurrentForm = async (videoPath: string) => {
    // 从路径中提取文件名
    const videoBaseName = videoPath.split(/[/\\]/).pop() || videoPath
    const videoNameWOExtension = videoBaseName.replace(/\.[^/.]+$/, '').slice(0, 80)
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
    if (!currentForm.value) {
        return 0 // 没有当前模板，跳过添加
    }

    const existingFile = currentForm.value.videos.find(
        f => f.path === videoPath || videoNameWOExtension === f.title
    )
    if (existingFile) {
        return 0 // 跳过已存在的文件
    }

    const currentAddedVideos = currentForm.value.videos.filter(video => {
        return (
            (video.finished_at && video.finished_at > 0) || (video.path && video.path.trim() !== '')
        )
    })

    // 检查是否超过100个视频的限制
    if (currentAddedVideos.length >= 100) {
        utilsStore.showMessage('单次提交最大限制100个视频文件，无法添加更多视频', 'error')
        return 0
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
            await uploadStore.createUploadTask(
                selectedUser.value.uid,
                currentTemplateName.value,
                currentForm.value.videos
            )
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
        utilsStore.showMessage('请先选择用户和模板后再拖拽文件', 'warning')
        return
    }

    // 添加视频文件到当前模板
    let addedCount = 0
    templateLoading.value = true
    for (const videoPath of videoFiles.paths) {
        addedCount += await addVideoToCurrentForm(videoPath)
    }
    templateLoading.value = false

    if (addedCount > 0) {
        utilsStore.showMessage(`成功添加 ${addedCount} 个视频文件`, 'success')
    } else {
        utilsStore.showMessage('所有文件都已存在，未添加新文件', 'info')
    }
}

// 处理登录成功
const handleLoginSuccess = async () => {
    showLoginDialog.value = false
    utilsStore.showMessage('登录成功', 'success')

    await userConfigStore.saveConfig()
    // 刷新所有数据
    await refreshAllData()
}

// 处理登录对话框关闭
const handleLoginDialogClose = async (done: () => void) => {
    if (loginLoading.value) {
        utilsStore.showMessage('登录过程中无法取消', 'warning')
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

// 处理用户展开按钮点击 - 在模板加载时禁用
const handleUserExpansion = (userUid: number) => {
    if (!templateLoading.value) {
        toggleUserExpanded(userUid)
    }
}

// 处理模板选择点击 - 在模板加载时禁用
const handleTemplateSelection = (user: any, templateName: string) => {
    if (!templateLoading.value) {
        selectTemplate(user, templateName)
    }
}

// 处理模板名编辑点击 - 在模板加载时禁用
const handleTemplateNameEdit = () => {
    if (!templateLoading.value) {
        startEditTemplateName()
    }
}

// 处理封面选择点击 - 在模板加载时禁用
const handleCoverSelection = () => {
    if (!templateLoading.value) {
        selectCoverWithTauri()
    }
}

// 选择模板
const selectTemplate = async (user: any, templateName: string) => {
    // 如果正在加载模板，禁止切换
    if (templateLoading.value) {
        return
    }

    if (selectedUser.value === user && currentTemplateName.value === templateName) {
        // 如果已经选择了相同的用户和模板，则不需要切换
        return
    }

    templateLoading.value = true
    try {
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
        await loadTemplate()

        // 保存模板选择到localStorage
        saveTemplateSelection(user.uid, templateName)

        // 如果模板有 aid，主动刷新模板数据
        const aid = currentTemplate.value?.aid
        setTimeout(async () => {
            if (aid) {
                try {
                    if (
                        selectedUser.value?.uid === user.uid &&
                        currentTemplateName.value === templateName
                    ) {
                        const newTemplate = await getNewTemplateFromAv(user.uid, aid)
                        const currentTemplateData =
                            userConfigStore.configRoot?.config[user.uid].templates[templateName]
                        if (
                            currentTemplateData &&
                            hasUnsavedChanges(currentTemplateData, newTemplate)
                        ) {
                            await ElMessageBox.confirm(
                                `检测到本地模板内容与bilibili不一致，是否刷新？（此操作会丢失所有未保存的更改）`,
                                '',
                                {
                                    confirmButtonText: '刷新并继续',
                                    cancelButtonText: '不刷新，仅显示当前',
                                    type: 'info'
                                }
                            )
                            await reloadTemplateFromAV(user.uid, aid)
                        }
                    }
                } catch (error) {
                    console.error('自动刷新模板数据失败:', error)
                }
            }
        }, 666)
        console.log(`已切换到模板: ${user.username} - ${templateName}`)
    } catch (error) {
        console.error('切换模板失败:', error)
        utilsStore.showMessage(`切换模板失败: ${error}`, 'error')
    } finally {
        templateLoading.value = false
    }
}

const resetTemplate = async () => {
    if (!selectedUser.value || !currentTemplateName.value) {
        utilsStore.showMessage('请先选择用户和模板', 'warning')
        return
    }

    // 如果正在加载模板，禁止重置
    if (templateLoading.value) {
        utilsStore.showMessage('模板正在加载中，请稍后再试', 'warning')
        return
    }

    // 确认重置
    try {
        await ElMessageBox.confirm('确定要清除所有未保存的更改吗?', '', {
            confirmButtonText: '确定',
            cancelButtonText: '取消',
            type: 'warning'
        })

        templateLoading.value = true
        try {
            currentForm.value =
                JSON.parse(
                    JSON.stringify(
                        userConfigStore.configBase?.config[selectedUser.value.uid]?.templates[
                            currentTemplateName.value
                        ]
                    )
                ) || userConfigStore.createDefaultTemplate()
            utilsStore.showMessage('模板已重置', 'success')
        } finally {
            templateLoading.value = false
        }
    } catch (error) {
        // 用户取消了重置
        console.log('重置操作已取消')
    }
}

const getNewTemplateFromAv = async (userUid: number, aid: number) => {
    try {
        const newTemplate = (await utilsStore.getVideoDetail(userUid, aid.toString())) as any

        // 处理视频列表
        if (newTemplate.videos && Array.isArray(newTemplate.videos)) {
            for (const video of newTemplate.videos) {
                video.id = video.filename
                video.path = ''
            }
        }

        if (newTemplate.aid && (await utilsStore.getSeasonList(userUid))) {
            const season_id = await utilsStore.getVideoSeason(userUid, newTemplate.aid)

            if (season_id !== 0) {
                const section_id = await utilsStore.seasonlist.find(
                    item => item.season_id === season_id
                )?.section_id
                newTemplate.season_id = season_id
                newTemplate.section_id = section_id
            }
        }

        newTemplate.watermark = currentForm.value?.watermark
        return newTemplate
    } catch (error) {
        console.error('获取新模板失败: ', error)
        throw error
    }
}

const reloadTemplateFromAV = async (userUid: number, aid: number) => {
    // 如果正在加载模板，禁止重新加载
    if (templateLoading.value) {
        return
    }

    if (!selectedUser.value || selectedUser.value.uid !== userUid) {
        return
    }

    if (!currentForm.value || currentForm.value.aid !== aid) {
        return
    }

    templateLoading.value = true
    try {
        const newTemplate = await getNewTemplateFromAv(userUid, aid)
        currentForm.value = newTemplate
        utilsStore.showMessage('模板数据已刷新', 'success')
    } catch (error) {
        console.error('刷新失败: ', error)
        utilsStore.showMessage(`刷新失败: ${error}`, 'error')
        throw error
    } finally {
        templateLoading.value = false
    }
}

// 加载模板数据到表单
const loadTemplate = async () => {
    try {
        // 如果没有模板，则使用默认模板配置
        if (!currentTemplate.value) {
            const defaultTemplate = userConfigStore.createDefaultTemplate()
            // 直接设置到配置中
            if (
                selectedUser.value &&
                currentTemplateName.value &&
                userConfigStore.configRoot?.config
            ) {
                const userConfig = userConfigStore.configRoot.config[selectedUser.value.uid]
                if (userConfig) {
                    userConfig.templates[currentTemplateName.value] = defaultTemplate
                }
            }

            // 清空标签
            tags.value = []

            // 清空分区选择
            selectedCategory.value = null
            selectedSubCategory.value = null

            // 等待所有更新完成
            await nextTick()

            return
        }

        const template = currentTemplate.value

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

        // 等待所有更新完成
        await nextTick()

        // 模板数据已直接操作，无需保存基础状态
    } catch (error) {
        console.error('加载模板失败:', error)
        utilsStore.showMessage(`加载模板失败: ${error}`, 'error')
    }
}

// 处理模板命令
const handleTemplateCommand = async (command: string, user: any, template: any) => {
    switch (command) {
        case 'duplicate':
            try {
                const newName = `${template.name}_副本`
                await userConfigStore.duplicateUserTemplate(user.uid, template.name, newName)
                utilsStore.showMessage('模板复制成功', 'success')
            } catch (error) {
                console.error('复制模板失败: ', error)
                utilsStore.showMessage(`'复制模板失败: ${error}'`, 'error')
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
                    utilsStore.showMessage('该名称的模板已存在，请使用其他名称', 'error')
                    return
                }

                // 获取原模板配置
                const originalTemplate = userConfigStore.getUserTemplate(user.uid, template.name)
                if (!originalTemplate) {
                    utilsStore.showMessage('原模板不存在', 'error')
                    return
                }

                // 先添加新模板
                await userConfigStore.addUserTemplate(user.uid, trimmedName, originalTemplate)

                // 再删除原模板
                await userConfigStore.removeUserTemplate(user.uid, template.name)

                // 更新当前选择
                if (
                    selectedUser.value?.uid === user.uid &&
                    currentTemplateName.value === template.name
                ) {
                    currentTemplateName.value = trimmedName
                    // 更新localStorage中的模板选择记录
                    saveTemplateSelection(user.uid, trimmedName)
                }

                utilsStore.showMessage('模板重命名成功', 'success')
            } catch (error) {
                if (error !== 'cancel') {
                    console.error('重命名模板失败: ', error)
                    utilsStore.showMessage(`'重命名模板失败: ${error}'`, 'error')
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
                    // 清除localStorage中的模板选择记录
                    localStorage.removeItem(TEMPLATE_SELECTION_KEY)
                }

                utilsStore.showMessage('模板删除成功', 'success')
            } catch (error) {
                if (error !== 'cancel') {
                    console.error('删除模板失败: ', error)
                    utilsStore.showMessage(`'删除模板失败: ${error}'`, 'error')
                }
            }
            break
    }
}

// 处理模板创建成功事件
const handleTemplateCreated = async (userUid: number, templateName: string) => {
    if (getCurrentAutoSubmitting.value) {
        return
    }

    // 自动选择新创建的模板
    const targetUser = loginUsers.value.find(user => user.uid === userUid)
    if (targetUser) {
        selectedUser.value = targetUser
        currentTemplateName.value = templateName

        // 滚动到顶部
        nextTick(() => {
            if (contentWrapperRef.value) {
                contentWrapperRef.value.scrollTop = 0
            }
        })

        templateLoading.value = true
        await loadTemplate()
        templateLoading.value = false

        // 保存新创建的模板选择
        saveTemplateSelection(userUid, templateName)
    }
}

// 保存模板
const saveTemplate = async () => {
    if (!selectedUser.value || !currentTemplateName.value || !currentTemplate.value) {
        utilsStore.showMessage('请先选择模板', 'error')
        return
    }

    try {
        // 直接保存当前模板配置
        await userConfigStore.updateUserTemplate(
            selectedUser.value.uid,
            currentTemplateName.value,
            currentTemplate.value
        )

        // 模板数据已直接操作并保存，无需额外状态管理
    } catch (error) {
        console.error('保存模板失败: ', error)
        utilsStore.showMessage(`'保存模板失败: ${error}'`, 'error')
    }
}

// 分区选择相关
const onCategoryChange = (categoryId: number) => {
    const category = typeList.value.find(item => item.id === categoryId)
    selectedCategory.value = category
    selectedSubCategory.value = null
    if (currentForm.value) {
        currentForm.value.tid = 0
        // 上一行会触发watch事件，导致selectedCategory被清空
        nextTick(() => {
            selectedCategory.value = category
        })
    }
}

const onSubCategoryChange = (subCategoryId: number) => {
    if (selectedCategory.value && selectedCategory.value.children) {
        const subCategory = selectedCategory.value.children.find(
            (item: any) => item.id === subCategoryId
        )
        selectedSubCategory.value = subCategory
        if (currentForm.value) {
            currentForm.value.tid = subCategoryId
        }
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
            utilsStore.showMessage('未选择任何封面文件', 'warning')
            return
        }

        if (selectedUser.value && currentTemplate.value && currentForm.value) {
            coverLoading.value = true
            templateLoading.value = true
            const url = await utilsStore.uploadCover(selectedUser.value.uid, selected)
            if (url) {
                currentTemplate.value.cover = url
                currentForm.value.cover = url
            } else {
                throw new Error('封面上传失败')
            }
        } else {
            utilsStore.showMessage('请先选择用户和模板', 'error')
        }
    } catch (error) {
        console.error('封面选择失败: ', error)
        utilsStore.showMessage(`'封面选择失败: ${error}'`, 'error')
        return
    } finally {
        coverLoading.value = false
        templateLoading.value = false
    }
}

// 使用 Tauri 文件对话框选择视频文件
const selectVideoWithTauri = async () => {
    if (templateLoading.value) {
        utilsStore.showMessage('模板加载中', 'warning')
        return
    }

    templateLoading.value = true
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

            utilsStore.showMessage(`已选择 ${added} 个文件`, 'success')
        } else if (typeof selected === 'string') {
            added += await addVideoToCurrentForm(selected)
            utilsStore.showMessage(`已选择 ${added} 个文件`, 'success')
        }
    } catch (error) {
        console.error('文件选择失败: ', error)
        utilsStore.showMessage(`'文件选择失败: ${error}'`, 'error')
    } finally {
        templateLoading.value = false
    }
}

// 清空所有文件
const clearAllVideos = async () => {
    if (!currentForm.value?.videos || currentForm.value.videos.length === 0) {
        return
    }

    const videoCount = currentForm.value.videos.length
    const videoText = videoCount === 1 ? '1 个文件' : `${videoCount} 个文件`

    templateLoading.value = true
    try {
        await ElMessageBox.confirm(`确定要清空所有已选择的 ${videoText} 吗？`, '确认清空文件', {
            confirmButtonText: '确定清空',
            cancelButtonText: '取消',
            type: 'warning',
            dangerouslyUseHTMLString: false
        })

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
        utilsStore.showMessage(`已清空 ${videoText}`, 'success')
    } catch {
        // 用户取消了操作
    } finally {
        templateLoading.value = false
    }
}

// 调整视频顺序
const removeUploadedFile = async (videoId: string) => {
    if (!currentForm.value?.videos) {
        return
    }

    templateLoading.value = true
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

            utilsStore.showMessage('文件删除成功', 'success')
        } catch (error) {
            // 如果用户取消了确认框，不显示错误消息
            if (error !== 'cancel') {
                console.error('删除文件失败:', error)
                utilsStore.showMessage(`删除文件失败: ${error}`, 'error')
            }
        }
    }
    templateLoading.value = false
}

// 上传相关
const createUpload = async () => {
    // 检查是否有文件可上传
    const hasUploadedFiles = currentForm.value?.videos && currentForm.value.videos.length > 0

    if (!hasUploadedFiles) {
        utilsStore.showMessage('请先选择视频文件', 'error')
        return
    }

    if (!selectedUser.value) {
        utilsStore.showMessage('请先选择用户', 'error')
        return
    }

    uploading.value = true
    try {
        if (currentForm.value) {
            console.log('开始上传文件:', currentForm.value.videos)
            // 确保传递的是正确格式的数组
            const num_added = await uploadStore.createUploadTask(
                selectedUser.value.uid,
                currentTemplateName.value,
                currentForm.value.videos
            )
            utilsStore.showMessage(`添加 ${num_added} 个文件到上传队列`, 'success')
        }

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
        utilsStore.showMessage(`上传失败: ${error}`, 'error')
    } finally {
        uploading.value = false
    }
}

// 处理文件夹监控添加视频事件
const handleAddVideosToForm = async (newVideos: any[]) => {
    templateLoading.value = true
    for (const videoPath of newVideos) {
        try {
            await addVideoToCurrentForm(videoPath)
        } catch (error) {
            console.error(`添加视频失败: ${videoPath}`, error)
        }
    }
    templateLoading.value = false
}

// 处理文件夹监控提交稿件事件
const handleSubmitTemplate = async () => {
    await submitTemplate()
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
    if (!currentForm.value?.videos || currentForm.value.videos.length === 0) {
        return false
    }
    return currentForm.value.videos.every(video => video.complete && video.path === '')
})

// 提交视频
const submitTemplate = async () => {
    if (!currentTemplateName.value || !selectedUser.value) {
        utilsStore.showMessage('请选择模板', 'error')
        return
    }

    if (!allFilesUploaded.value) {
        const currentAutoSubmitting = getCurrentAutoSubmitting.value
        if (!currentAutoSubmitting) {
            // 首次点击，开始自动提交
            // 将当前video列表加入upload queue
            try {
                if (currentForm.value?.videos && currentForm.value.videos.length > 0) {
                    await uploadStore.createUploadTask(
                        selectedUser.value.uid,
                        currentTemplateName.value,
                        currentForm.value.videos
                    )

                    setTimeout(async () => {
                        try {
                            await autoStartWaitingTasks()
                        } catch (error) {
                            console.error('自动开始任务失败:', error)
                        }
                    }, 500)
                }
            } catch (error) {
                console.error('添加到上传队列失败:', error)
                utilsStore.showMessage(`添加到上传队列失败: ${error}`, 'error')
            }
            setAutoSubmitting(selectedUser.value.uid, currentTemplateName.value, true)
            startAutoSubmitCheck()
            utilsStore.showMessage('已启动自动提交，上传完成后将自动提交', 'info')
        } else {
            // 第二次点击，取消自动提交
            setAutoSubmitting(selectedUser.value.uid, currentTemplateName.value, false)
            utilsStore.showMessage('已取消自动提交', 'info')
        }
        return
    } else {
        performTemplateSubmit(selectedUser.value.uid, currentTemplateName.value, currentForm.value)
    }
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
        utilsStore.showMessage('模板名称不能为空', 'error')
        cancelEditTemplateName()
        return
    }

    if (newName === currentTemplateName.value) {
        cancelEditTemplateName()
        return
    }

    if (!selectedUser.value) {
        utilsStore.showMessage('未选择用户', 'error')
        cancelEditTemplateName()
        return
    }

    try {
        // 检查是否已存在同名模板
        const existingTemplate = userConfigStore.getUserTemplate(selectedUser.value.uid, newName)
        if (existingTemplate) {
            utilsStore.showMessage('该名称的模板已存在，请使用其他名称', 'error')
            return
        }

        // 获取原模板配置
        const originalTemplate = userConfigStore.getUserTemplate(
            selectedUser.value.uid,
            currentTemplateName.value
        )
        if (!originalTemplate) {
            utilsStore.showMessage('原模板不存在', 'error')
            cancelEditTemplateName()
            return
        }

        // 先添加新模板
        await userConfigStore.addUserTemplate(selectedUser.value.uid, newName, originalTemplate)

        // 再删除原模板
        await userConfigStore.removeUserTemplate(selectedUser.value.uid, currentTemplateName.value)

        // 更新当前选择
        currentTemplateName.value = newName

        utilsStore.showMessage('模板重命名成功', 'success')
        isEditingTemplateName.value = false
    } catch (error) {
        console.error('重命名模板失败: ', error)
        utilsStore.showMessage(`重命名模板失败: ${error}`, 'error')
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
        utilsStore.showMessage('用户有未完成的上传任务，无法登出', 'success')
        return
    }

    try {
        const success = await authStore.logoutUser(uid)
        if (success) {
            // 如果登出的用户正是当前选择的用户，清除相关记录
            if (selectedUser.value?.uid === uid) {
                selectedUser.value = null
                currentTemplateName.value = ''
                localStorage.removeItem(TEMPLATE_SELECTION_KEY)
            } else {
                // 检查localStorage中记录的用户是否是被登出的用户
                try {
                    const saved = localStorage.getItem(TEMPLATE_SELECTION_KEY)
                    if (saved) {
                        const selection = JSON.parse(saved)
                        if (selection.userUid === uid) {
                            localStorage.removeItem(TEMPLATE_SELECTION_KEY)
                        }
                    }
                } catch (error) {
                    console.error('清理localStorage记录失败:', error)
                }
            }

            utilsStore.showMessage('用户已登出', 'success')
            // 刷新前端数据
            await refreshAllData()
        } else {
            utilsStore.showMessage('登出失败', 'error')
        }
    } catch (error) {
        // 如果用户取消了确认框，error会是'cancel'，不需要显示错误
        if (error !== 'cancel') {
            console.error('登出用户失败:', error)
            utilsStore.showMessage(`登出失败: ${error}`, 'error')
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

// 导出日志
const exportLogs = async () => {
    try {
        const zipPath = await utilsStore.exportLogs()
        const zipName = zipPath.split(/[/\\]/).pop() || zipPath

        const savePath = await save({
            defaultPath: zipName,
            filters: [{ name: 'ZIP', extensions: ['zip'] }]
        })

        if (savePath) {
            // 复制 ZIP 文件到用户指定位置
            await copyFile(zipPath, savePath)
            await remove(zipPath)
            console.log('文件已保存到：', savePath)
        }
    } catch (error) {
        console.error('导出日志失败:', error)
    }
}

// 检查视频转码状态
const checkVideoStatus = async () => {
    if (!selectedUser.value || !currentTemplate.value?.aid) return

    try {
        // 先刷新模板数据
        await ElMessageBox.confirm(
            `此操作会重新拉取模板数据，此操作会丢失未保存的更改，是否继续？`,
            '',
            {
                confirmButtonText: '刷新并继续',
                cancelButtonText: '不刷新，仅显示当前',
                type: 'info'
            }
        )
        await reloadTemplateFromAV(selectedUser.value.uid, currentTemplate.value.aid)
        // 然后显示状态对话框
        showVideoStatusDialog.value = true
    } catch (error) {
        console.error('刷新模板数据失败:', error)
        // 即使刷新失败也显示对话框
        showVideoStatusDialog.value = true
    }
}

// 检查更新
const checkUpdate = async () => {
    try {
        const updateInfo = await utilsStore.checkUpdate()
        if (updateInfo) {
            // 如果有更新，显示确认对话框
            try {
                await ElMessageBox.confirm(`发现新版本 ${updateInfo}，是否前往下载？`, '发现更新', {
                    confirmButtonText: '前往下载',
                    cancelButtonText: '稍后再说',
                    type: 'info'
                })
                // 用户确认后打开下载页面
                await openUrl(`https://github.com/biliup/biliup-app-new/releases/tag/${updateInfo}`)
            } catch {
                // 用户取消，不做任何操作
            }
        } else {
            utilsStore.showMessage('当前已是最新版本', 'success')
        }
    } catch (error) {
        console.error('检查更新失败:', error)
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
    display: inline-block;
}

.app-version {
    display: inline-block;
}

.header-center {
    display: flex;
    align-items: center;
    gap: 12px;
}

.header-right {
    display: flex;
    align-items: center;
    gap: 20px;
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
    align-items: center;
}

.highlight-checkbox {
    margin-right: 8px;
    font-size: 12px;
}

.highlight-checkbox :deep(.el-checkbox__label) {
    font-size: 12px;
    color: #606266;
}

.highlight-checkbox-text {
    line-height: 1.2;
    text-align: center;
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

.template-item.auto-submitting {
    position: relative;
    overflow: hidden;
    background: linear-gradient(45deg, #e3f2fd, #f3e5f5);
    border: 2px solid #409eff;
    box-shadow: 0 0 20px rgba(64, 158, 255, 0.4);
    animation: pulse-border 1.5s ease-in-out infinite alternate;
}

.template-item.auto-submitting::before {
    content: '';
    position: absolute;
    top: 0;
    left: -100%;
    width: 100%;
    height: 100%;
    background: linear-gradient(90deg, transparent, rgba(64, 158, 255, 0.6), transparent);
    animation: shimmer 1.5s infinite;
    z-index: 1;
}

.template-item.auto-submitting::after {
    content: '⚡ 自动上传中...';
    position: absolute;
    top: 2px;
    right: 6px;
    font-size: 10px;
    color: #409eff;
    font-weight: bold;
    animation: blink 1s infinite;
    z-index: 3;
}

.template-item.auto-submitting .template-main {
    position: relative;
    z-index: 2;
}

.template-item.auto-submitting .template-name {
    color: #1976d2;
    font-weight: 600;
    text-shadow: 0 1px 3px rgba(25, 118, 210, 0.2);
}

@keyframes shimmer {
    0% {
        left: -100%;
    }
    100% {
        left: 100%;
    }
}

@keyframes pulse-border {
    0% {
        border-color: #409eff;
        box-shadow: 0 0 20px rgba(64, 158, 255, 0.4);
    }
    100% {
        border-color: #1890ff;
        box-shadow: 0 0 30px rgba(24, 144, 255, 0.6);
    }
}

@keyframes blink {
    0%,
    50% {
        opacity: 1;
    }
    51%,
    100% {
        opacity: 0.3;
    }
}

.template-item.auto-submitting-simple {
    border: 2px solid #409eff !important;
    background: rgba(64, 158, 255, 0.05) !important;
    position: relative;
}

.template-item.auto-submitting-simple::after {
    content: '⚡ 自动上传中...';
    position: absolute;
    top: 2px;
    right: 6px;
    font-size: 10px;
    color: #409eff;
    font-weight: bold;
    z-index: 3;
}

.template-item.template-loading {
    position: relative;
    background: #f5f7fa;
    border: 2px solid #e4e7ed;
    cursor: not-allowed;
    opacity: 0.8;
}

.template-item.template-loading::before {
    content: '';
    position: absolute;
    top: 0;
    left: 0;
    width: 100%;
    height: 100%;
    background: linear-gradient(90deg, transparent, rgba(230, 244, 255, 0.8), transparent);
    animation: loading-shimmer 1.5s infinite;
    z-index: 1;
}

.template-item.template-loading::after {
    content: '🔄 加载中...';
    position: absolute;
    top: 2px;
    right: 6px;
    font-size: 10px;
    color: #909399;
    font-weight: bold;
    z-index: 3;
    animation: loading-spin 1s linear infinite;
}

.template-item.template-loading .template-main {
    position: relative;
    z-index: 2;
}

@keyframes loading-shimmer {
    0% {
        transform: translateX(-100%);
    }
    100% {
        transform: translateX(100%);
    }
}

@keyframes loading-spin {
    0% {
        opacity: 1;
    }
    50% {
        opacity: 0.5;
    }
    100% {
        opacity: 1;
    }
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

.edit-bv-template-disaplay {
    display: inline-block;
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

.refresh-btn {
    cursor: pointer;
    color: #606266;
    font-size: 16px;
    transition: all 0.3s;
    border-radius: 4px;
}

.refresh-btn:hover {
    color: #409eff;
    background-color: #f0f9ff;
    transform: rotate(180deg);
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
    height: 10px;
}

.card-header:hover {
    color: #409eff;
}

.card-header .header-actions {
    display: flex;
    gap: 8px;
    align-items: center;
}

.card-header .header-actions .el-button {
    margin: 0;
    padding: 4px;
    border: none;
    background: transparent;
    transition: all 0.3s ease;
}

.card-header .header-actions .el-button:hover {
    background: rgba(245, 108, 108, 0.1);
    color: #f56c6c;
    transform: scale(1.1);
}

.card-header .header-actions .el-button .el-icon {
    font-size: 14px;
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

/* 禁用状态样式 */
.cover-uploader.disabled {
    cursor: not-allowed !important;
    opacity: 0.6 !important;
}

.cover-uploader.disabled:hover {
    border-color: #dcdfe6 !important;
}

.template-name-display.disabled {
    cursor: not-allowed !important;
    opacity: 0.6 !important;
    color: #909399 !important;
}

.template-name-display.disabled .edit-hint-icon {
    color: #c0c4cc !important;
}

/* 用户头部禁用状态 */
.user-header.disabled {
    cursor: not-allowed !important;
    opacity: 0.6 !important;
}

.user-header.disabled:hover {
    background: #fff !important;
}

/* 模板项禁用状态 */
.template-item.disabled {
    cursor: not-allowed !important;
    opacity: 0.6 !important;
}

.template-item.disabled:hover {
    background: #fff !important;
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
