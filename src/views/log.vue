<script setup lang="ts">

import { info, } from '@tauri-apps/plugin-log';
import { invoke } from '@tauri-apps/api/core';
import { ref, nextTick } from 'vue';
import { ElMessage } from 'element-plus';


const task_list = ref<Array<Record<string, any>>>([]);
const sku = ref('');
const ps_module = ref('');
const drawerVisible = ref(false);
const drawerTitle = ref('日志详情');
const currentTaskId = ref<string | null>(null);
const logs = ref<Array<{ log_time: string, message: string }>>([]);
const logsLoading = ref(false);

async function copyLog(l: { log_time: string, message: string }, e: MouseEvent) {
    const text = `[${l.log_time}] ${l.message}`;
    const el = (e.currentTarget as HTMLElement)?.querySelector('.msg') as HTMLElement | null; 
    const isTruncated = el ? el.scrollWidth > el.clientWidth : true;
    if (!isTruncated) return
    try {
        await navigator.clipboard.writeText(text);
        ElMessage.success('日志已复制到剪贴板');
    } catch {
        try {
            const mod: any = await import('@tauri-apps/plugin-clipboard-manager').catch(() => null);
            if (mod?.writeText) {
                await mod.writeText(text);
                ElMessage.success('日志已复制到剪贴板');
            } else {
                throw new Error('不支持复制到剪贴板');
            }
        } catch {
            ElMessage.error('复制日志失败，请手动复制');
        }
    }
}


async function viewLogs(task_id: string) {
    drawerVisible.value = true;
    drawerTitle.value = `日志详情 - 任务ID: ${task_id}`;
    currentTaskId.value = task_id;
    logsLoading.value = true;
    try {
        const resp = await invoke<string>('get_task_logs', { taskId: task_id });
        const json = JSON.parse(resp);
        console.log(json)
        logs.value = json.data ?? json.items ?? []
        await nextTick()
        const sc = document.querySelector('.log-scroll') as HTMLElement;
        if (sc) {
            sc.scrollTop = sc.scrollHeight;
        }
    } finally {
        logsLoading.value = false;
    }
}

async function refreshLogs() {
    if (currentTaskId.value) {
        await viewLogs(currentTaskId.value);
    }
}


async function fetchLogs() {
    const s = sku.value.trim();
    const m = ps_module.value.trim();

    const payload: Record<string, any> = {}
    if (s) payload.sku = s;
    if (m) payload.module = m;

    const resp = await invoke<string>('get_task_list', payload);
    const json = JSON.parse(resp);
    console.log(json.data ?? json.items ?? []);
    task_list.value = json.data ?? json.items ?? [];
    info(`查询日志成功, 条件: ${JSON.stringify(payload)}, 结果数: ${task_list.value.length}`);
}

</script>
<template>
    <div>
        <div class="search_bar">
            <div>
                <span>SKU：</span>
                <el-input v-model="sku" placeholder="请输入SKU" style="width: 200px; margin-right: 10px;"></el-input>
            </div>
            <div>
                <span>PS模版</span>
                <el-input v-model="ps_module" placeholder="请输入PS模版"
                    style="width: 200px; margin-right: 10px;"></el-input>
            </div>
            <el-button type="primary" @click="fetchLogs">查询</el-button>
        </div>
        <el-table :data="task_list" style="width: 100%" row-key="task_id">
            <!-- <el-table-column prop="task_id" label="任务ID"  /> -->
            <el-table-column prop="run_time" label="执行时间" width="180" />
            <el-table-column prop="SKU" label="SKU" width="180" />
            <el-table-column prop="module" label="PS模版" width="180" />
            <el-table-column prop="status" label="状态" width="120">
                <template #default="{ row }">
                    <span v-if="row.status === 0">失败</span>
                    <span v-else-if="row.status === 1">成功</span>
                    <span v-else>未知</span>
                </template>
            </el-table-column>
            <el-table-column lable="操作">
                <template #default="{ row }">
                    <el-button type="text" @click="viewLogs(row.task_id)">查看日志</el-button>
                </template>
            </el-table-column>
        </el-table>
        <el-drawer v-model="drawerVisible" :title="drawerTitle" direction="rtl" size="80%" destory-on-close>
            <div class="log-toolbar">
                <el-button size="small" @click="refreshLogs" :loading="logsLoading">刷新</el-button>
            </div>
            <div class="log-scroll">
                <template v-if="!logsLoading">
                    <!-- 改为普通行展示，超长省略，悬浮显示完整 -->
                    <div class="log-list">
                        <div class="log-item" v-for="(l, i) in logs" :key="i" :title="`[${l.log_time}] ${l.message}`" @click="copyLog(l, $event)">
                            <span class="time">[{{ l.log_time }}]</span>
                            <span class="msg">{{ l.message }}</span>
                        </div>
                    </div>
                    <div v-if="logs.length === 0" class="log-empty">暂无日志</div>
                </template>
                <div v-else class="log-empty">加载中...</div>
            </div>
        </el-drawer>
    </div>
</template>
<style scoped>
.search_bar {
    display: flex;
    align-items: center;
    margin-bottom: 20px;
}

.log-toolbar {
    display: flex;
    justify-content: flex-end;
    padding-bottom: 8px;
}

.log-scroll {
    height: calc(100vh - 220px);
    overflow: auto;
    border: 1px solid #eee;
    border-radius: 6px;
}



.log-empty {
    color: #999;
    padding: 12px;
    text-align: center;
}

.log-list {
    background: #0b1221;
    color: #c7d2fe;
    padding: 10px;
    border-radius: 6px;
    min-height: 200px;
    font-family: ui-monospace, SFMono-Regular, Consolas, 'Courier New', monospace;
}

.log-item {
    display: flex;
    align-items: center;
    gap: 8px;
    line-height: 1.6;
}

.log-item .time {
    color: #8ab4f8;
    flex: 0 0 auto;
}

.log-item .msg {
    flex: 1 1 auto;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    /* 超长省略 */

}
</style>