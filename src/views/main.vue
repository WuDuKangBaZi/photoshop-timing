<script lang="ts" setup>
import { info, error } from '@tauri-apps/plugin-log';
import { invoke } from '@tauri-apps/api/core';
import { message } from '@tauri-apps/plugin-dialog';
import { load, Store } from '@tauri-apps/plugin-store';
import { listen } from '@tauri-apps/api/event';
import { onMounted, ref } from 'vue';
import parser from 'cron-parser';

let store: Store | undefined;
const timing_type = ref();
const timing_values = ref();
const executeButtonType = ref('success');
const executeButtonText = ref('执行');
const executeButtonDisabled = ref(false);
const run_tooltip = ref("执行单次任务");
let isListenerRegistered = false;
const settingsDisabled = ref(false);


onMounted(async () => {
    store = await load('store.bin', {});
    get_cron();
    if (!isListenerRegistered) {
        isListenerRegistered = true;
        listen('timing_action', (event) => {
            if (event.payload === 'update_timing') {
                get_cron();
            }
        });
    }
});

function getNextRunTime(cronExpression: string): Date | null {
    try {
        if (timing_type.value == 'specify') {
            console.log('定时任务类型: 特定时间');
            info(`定时任务类型: 特定时间, cron表达式: ${cronExpression}`);
            return new Date(timing_values.value);
        }
        console.log('解析Cron表达式:', cronExpression);
        info(`解析Cron表达式: ${cronExpression}`);
        const interval = parser.parse(cronExpression);
        return interval.next().toDate();
    } catch (e) {
        error(`Cron表达式解析错误: ${e}`);
        console.error('Cron表达式解析错误:', e);
        return null;
    }
}

async function get_cron() {
    if (!store) {
        console.error('Store未初始化');
        error('Store未初始化，请稍后再试!');
        return;
    }
    timing_type.value = await store.get('timing_type');
    timing_values.value = await store.get('timing_cron');
    console.log("cron: =>", timing_values);
    if (timing_values.value === null || timing_values.value === undefined) {
        run_tooltip.value = "执行单次任务";
        executeButtonText.value = '执行';
        info('开始执行单次任务');

    } else {
        const nextRun = getNextRunTime(timing_values.value);
        console.log('下次运行时间:', nextRun);
        info(`下次运行时间: ${nextRun}`);
        run_tooltip.value = "执行定时任务"
        executeButtonText.value = '开始定时'
    }

}



async function execute(event: MouseEvent) {
    const target = event.target as HTMLElement;
    if (target.textContent === '执行') {
        // await message('正在执行', { title: "Photoshop自动化", kind: "info" });
        try {
            executeButtonText.value = '停止';
            executeButtonType.value = 'danger';
            executeButtonDisabled.value = true;
            const invo_result = await invoke('execute_task')
            info(`任务执行结果: ${invo_result}`);
            target.removeAttribute('disabled');
            executeButtonText.value = '执行';
            executeButtonType.value = 'success';
            executeButtonDisabled.value = false;
        }catch(e){
            error(`任务执行失败: ${e}`);
            await message('任务执行失败，请检查日志!', { title: "Photoshop自动化", kind: "error" });
            target.removeAttribute('disabled');
            executeButtonText.value = '执行';
            executeButtonType.value = 'success';
            executeButtonDisabled.value = false;
        }
        
    } else if(target.textContent === '开始定时'){
        settingsDisabled.value = true;
        await invoke('start_cron',{cron:timing_values.value as string || ''} );
        executeButtonText.value = '停止定时';

    }else if (target.textContent === '停止定时'){
        await invoke('stop_cron');
        settingsDisabled.value = false;
        executeButtonText.value = '开始定时';
    }

}

async function setting_timing() {
    // 这里可以添加打开定时设置窗口的逻辑
    await invoke('open_timing_window');
}

async function open_log() {
    await invoke('open_logs_window');
}
</script>
<template>
    <el-button class="timing" type="warning" @click="setting_timing" plain
        :disabled="settingsDisabled">设置定时</el-button>
    <el-button class="execute" :type="executeButtonType" @click="execute" plain :disabled="executeButtonDisabled">{{
        executeButtonText }}</el-button>
    <el-button class="logs" type="info" plain @click="open_log">查看日志</el-button>
</template>
<style scoped>
.logs {
    position: absolute;
    top: 10px;
    /* left: 10px; */
    height: 30px;
    width: 80px;
}

.timing {
    position: absolute;
    top: 10px;
    left: 110px;
    height: 30px;
    width: 80px;
}

.execute {
    position: absolute;
    top: 10px;
    left: 190px;
    height: 30px;
    width: 80px;
}
</style>