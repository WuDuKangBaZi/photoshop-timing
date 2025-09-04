<script lang="ts" setup>
import { ref, onMounted } from 'vue';
import { emit } from '@tauri-apps/api/event';
import { load, Store } from '@tauri-apps/plugin-store';
import { getCurrentWindow } from '@tauri-apps/api/window';

const timing_type = ref();
let store : Store | undefined;

const timing_values = ref({
    specify: { date: new Date() },
    minute: { interval: 1 },
    hour: { interval: 1, minute: 0 },
    day: { hours: 12, minute: 0 },
    week: {
        weeks: [1, 2, 3, 4, 5],
        hours: 12,
        minute: 0
    },
    month: {
        months: [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12],
        weeks: [1, 2, 3, 4, 5],
        hours: 12,
        minute: 0
    },
    senior: { value: "" }
})


function parseCronToTimingValues(cron: string) {
    console.log('解析Cron表达式:', cron);

    const parts = cron.split(' ');

    if (parts.length !== 5) {
        console.error('Cron表达式格式错误');
        return;
    }

    switch (timing_type.value) {
        case 'minute':
            // 每隔 X 分钟执行
            timing_values.value.minute.interval = parts[0].startsWith('*/')
                ? parseInt(parts[0].slice(2), 10)
                : 1;
            break;

        case 'hour':
            // 每隔 X 小时，在指定的分钟执行
            timing_values.value.hour.interval = parts[1].startsWith('*/')
                ? parseInt(parts[1].slice(2), 10)
                : 1;
            timing_values.value.hour.minute = parts[0] === '*' ? 0 : parseInt(parts[0], 10);
            break;

        case 'day':
            // 每天在指定的小时和分钟执行
            timing_values.value.day.hours = parts[1] === '*' ? 0 : parseInt(parts[1], 10);
            timing_values.value.day.minute = parts[0] === '*' ? 0 : parseInt(parts[0], 10);
            break;

        case 'week':
            // 每周的指定天，在指定的小时和分钟执行
            timing_values.value.week.weeks = parts[4] === '*'
                ? []
                : parts[4].split(',').map(Number);
            timing_values.value.week.hours = parts[1] === '*' ? 0 : parseInt(parts[1], 10);
            timing_values.value.week.minute = parts[0] === '*' ? 0 : parseInt(parts[0], 10);
            break;

        case 'month':
            // 每月的指定天和周，在指定的小时和分钟执行
            timing_values.value.month.months = parts[3] === '*'
                ? []
                : parts[3].split(',').map(Number);
            timing_values.value.month.weeks = parts[4] === '*'
                ? []
                : parts[4].split(',').map(Number);
            timing_values.value.month.hours = parts[1] === '*' ? 0 : parseInt(parts[1], 10);
            timing_values.value.month.minute = parts[0] === '*' ? 0 : parseInt(parts[0], 10);
            break;

        case 'senior':
            // 高级模式直接保存原始表达式
            timing_values.value.senior.value = cron;
            break;

        default:
            console.error('未知的 timing_type:', timing_type.value);
            break;
    }

    console.log('解析后的定时值:', timing_values.value);
}

function generateCronFromTimingValues() {
    let minute = '*';
    let hour = '*';
    let day = '*';
    let month = '*';
    let week = '*';

    switch (timing_type.value) {
        case 'specify':
            // 如果是 "specify"，直接返回 ISO 格式的日期字符串
            return timing_values.value.specify.date.toISOString();

        case 'minute':
            // 每隔 X 分钟执行
            minute = `*/${timing_values.value.minute.interval}`; // 每间隔 X 分钟
            break;

        case 'hour':
            console.log("每小时")
            // 每隔 X 小时，在指定的分钟执行
            hour = `*/${timing_values.value.hour.interval}`; // 每间隔 X 小时
            minute = timing_values.value.hour.minute.toString(); // 在指定分钟执行
            break;

        case 'day':
            // 每天在指定的小时和分钟执行
            hour = timing_values.value.day.hours.toString();
            minute = timing_values.value.day.minute.toString();
            break;

        case 'week':
            // 每周的指定天，在指定的小时和分钟执行
            week = timing_values.value.week.weeks.join(',');
            hour = timing_values.value.week.hours.toString();
            minute = timing_values.value.week.minute.toString();
            break;

        case 'month':
            // 每月的指定天和周，在指定的小时和分钟执行
            month = timing_values.value.month.months.join(',');
            week = timing_values.value.month.weeks.join(',');
            hour = timing_values.value.month.hours.toString();
            minute = timing_values.value.month.minute.toString();
            break;

        case 'senior':
            // 如果是高级模式，直接返回用户输入的自定义表达式
            return timing_values.value.senior.value;

        default:
            console.error('未知的 timing_type:', timing_type.value);
            return '';
    }

    // 拼接 cron 表达式
    const cronExpression = `${minute} ${hour} ${day} ${month} ${week}`;
    console.log('生成的 cron 表达式:', cronExpression);
    return cronExpression;
}


onMounted( async() => {
    store =  await load('store.bin', {});
    const timing_cron = await store.get('timing_cron');
    timing_type.value = await store.get('timing_type');
    if(!timing_type.value) {
        timing_type.value = 'specify';
        timing_values.value.specify.date = new Date();
    }
    parseCronToTimingValues(timing_cron as string);
})

function handle_timing_type_change() {
    console.log("timing_type changed:", timing_type.value);
}

function handle_timing_chanage() {
    console.log("timing_type changed:", timing_type.value);
    console.log("timing_values changed:", timing_values.value);
}
function clear_timing(){
    store?.delete('timing_cron');
    store?.delete('timing_type');
    store?.save();
    emit('timing_action', 'update_timing');
    getCurrentWindow().close();
}

function change_timing(){
    console.log("保存定时任务变更");
    let cron_value = generateCronFromTimingValues();
    store?.set('timing_cron', cron_value);
    store?.set('timing_type', timing_type.value);
    store?.save();
    emit('timing_action', 'update_timing');
    getCurrentWindow().close();
}
</script>
<template>
    <div class="timing-container">
        <div class="timing-type">
            <el-radio-group v-model="timing_type" class="timing-type-group" @change="handle_timing_type_change">
                <el-radio label="specify">某天</el-radio>
                <el-radio label="minute">分钟</el-radio>
                <el-radio label="hour">小时</el-radio>
                <el-radio label="day">每天</el-radio>
                <el-radio label="week">每周</el-radio>
                <el-radio label="month">每月</el-radio>
                <el-radio label="senior">高级</el-radio>
            </el-radio-group>
        </div>
        <div class="timing-center">
            <div v-if="timing_type === 'specify'" class="timing-selected">
                <el-date-picker v-model="timing_values.specify.date" type="datetime" placeholder="选择日期" append-to-body
                    @change="handle_timing_chanage" popper-class="custom-date-picker"></el-date-picker>
            </div>
            <div v-else-if="timing_type === 'minute'" class="timing-selected">
                <el-text>每&nbsp;&nbsp;</el-text>
                <el-input-number v-model="timing_values.minute.interval" controls-position="right" :min="1" :max="59"
                    @change="handle_timing_chanage" class="timing-option" style="width: 100px;"></el-input-number>
                <el-text>&nbsp;&nbsp;分钟执行</el-text>
            </div>
            <div v-else-if="timing_type === 'hour'" class="timing-selected">
                <el-text>每间隔&nbsp;</el-text>
                <el-input-number v-model="timing_values.hour.interval" @change="handle_timing_chanage" :min="1"
                    :max="23" controls-position="right" class="timing-option" style="width: 100px;"></el-input-number>
                <el-text>&nbsp;&nbsp;小时,在</el-text>
                <el-input-number v-model="timing_values.hour.minute" @change="handle_timing_chanage"
                    controls-position="right" class="timing-option" style="width: 100px;"></el-input-number>
                <el-text>&nbsp;&nbsp;分执行</el-text>
            </div>
            <div v-else-if="timing_type === 'day'" class="timing-selected">
                <el-text>每天在</el-text>
                <el-input-number v-model="timing_values.day.hours" @change="handle_timing_chanage" :min="0" :max="23"
                    controls-position="right" class="timing-option" style="width: 100px;"></el-input-number>
                <el-text>&nbsp;&nbsp;时</el-text>
                <el-input-number v-model="timing_values.day.minute" @change="handle_timing_chanage"
                    controls-position="right" class="timing-option" style="width: 100px;"></el-input-number>
                <el-text>&nbsp;&nbsp;分执行</el-text>
            </div>
            <div v-else-if="timing_type === 'week'" class="timing-selected other">
                <el-text>每周的</el-text>
                <el-checkbox-group v-model="timing_values.week.weeks" @change="handle_timing_chanage"
                    class="timing-option">
                    <el-checkbox :value="1">周一</el-checkbox>
                    <el-checkbox :value="2">周二</el-checkbox>
                    <el-checkbox :value="3">周三</el-checkbox>
                    <el-checkbox :value="4">周四</el-checkbox>
                    <el-checkbox :value="5">周五</el-checkbox>
                    <el-checkbox :value="6">周六</el-checkbox>
                    <el-checkbox :value="7">周日</el-checkbox>
                </el-checkbox-group>
                <div style="padding-top: 20px;">
                    <el-text>在&nbsp;&nbsp;</el-text>
                    <el-input-number v-model="timing_values.week.hours" @change="handle_timing_chanage" :min="0"
                        :max="23" controls-position="right" class="timing-option"
                        style="width: 100px;"></el-input-number>
                    <el-text>&nbsp;&nbsp;时</el-text>
                    <el-input-number v-model="timing_values.week.minute" @change="handle_timing_chanage"
                        controls-position="right" class="timing-option" style="width: 100px;"></el-input-number>
                    <el-text>&nbsp;&nbsp;分执行</el-text>
                </div>
            </div>
            <div v-else-if="timing_type === 'month'" class="timing-selected other">
                <el-text class="">每年的</el-text>
                <el-checkbox-group v-model="timing_values.month.months" @change="handle_timing_chanage"
                    class="timing-option">
                    <el-checkbox :value="1">1月</el-checkbox>
                    <el-checkbox :value="2">2月</el-checkbox>
                    <el-checkbox :value="3">3月</el-checkbox>
                    <el-checkbox :value="4">4月</el-checkbox>
                    <el-checkbox :value="5">5月</el-checkbox>
                    <el-checkbox :value="6">6月</el-checkbox>
                    <el-checkbox :value="7">7月</el-checkbox>
                    <el-checkbox :value="8">8月</el-checkbox>
                    <el-checkbox :value="9">9月</el-checkbox>
                    <el-checkbox :value="10">10月</el-checkbox>
                    <el-checkbox :value="11">11月</el-checkbox>
                    <el-checkbox :value="12">12月</el-checkbox>
                </el-checkbox-group>
                <el-text>每周的</el-text>
                <el-checkbox-group v-model="timing_values.month.weeks" @change="handle_timing_chanage"
                    class="timing-option">
                    <el-checkbox :value="1">周一</el-checkbox>
                    <el-checkbox :value="2">周二</el-checkbox>
                    <el-checkbox :value="3">周三</el-checkbox>
                    <el-checkbox :value="4">周四</el-checkbox>
                    <el-checkbox :value="5">周五</el-checkbox>
                    <el-checkbox :value="6">周六</el-checkbox>
                    <el-checkbox :value="7">周日</el-checkbox>
                </el-checkbox-group>
                <div style="padding-top: 5px;">
                    <el-text>在当天&nbsp;</el-text>
                    <el-input-number v-model="timing_values.month.hours" @change="handle_timing_chanage" :min="0"
                        :max="23" controls-position="right" class="timing-option"
                        style="width: 100px;"></el-input-number>
                    <el-text>&nbsp;&nbsp;时</el-text>
                    <el-input-number v-model="timing_values.month.minute" @change="handle_timing_chanage"
                        controls-position="right" class="timing-option" style="width: 100px;"></el-input-number>
                    <el-text>&nbsp;&nbsp;分</el-text>

                </div>
            </div>
        </div>
    </div>
    <div class="timing-options">
    <div style="height: 145px;"></div>
    <div class="timing-buttons">
        <el-button type="danger" style="text-align: right; margin-right: 10px;" plain @click="clear_timing">取消定时</el-button>
    <el-button type="primary" style="text-align: right; margin-right: 10px;" plain @click="change_timing">保存变更</el-button>
    </div>
    </div>
</template>
<style scoped>
.timing-buttons{
    display: flex;
    justify-content: flex-end;
    margin-top: 10px;
}
.timing-selected.other {
    display: flex;
    flex-direction: column;
    /* justify-content: left; */
    padding: 10px;
}
.timing-selected.other el-text {
    align-self:flex-start;
}
.timing-selected el-text {
    white-space: nowarp;
    align-items: flex-start;
}

.custom-date-picker {
    width: 200px !important;
    /* 设置弹出窗口的宽度 */
    height: 200px;
    font-size: 12px;
    /* 可选：调整字体大小 */
}

.timing-option {
    align-items: flex-start;
}

.timing-selected {
    display: flex;
    align-items: flex-start;
    justify-content: left;
    padding: 10px;
    /* 添加内边距 */
}

.timing-center {
    padding-left: 10px;
    
    height: 100%;
    flex: 1;
    display: flex;
    align-items: flex-start;
    margin-right: 0;
}

.timing-container {
    /* border: 1px solid #dcdfe6; */
    height: 265px;
    display: flex;
    flex-direction: row;
    align-items: flex-start;
    padding-left: 10px;
    padding-bottom: 5px;
}

.timing-type {
    height: 100%;
    flex: 0 0 15%;
    text-align: left;
    padding-top: 10px;
}

.timing-type-group {
    display: flex;
    flex-direction: column;
    /* 垂直排列 */
    gap: 90em;
    /* 设置选项之间的间距 */
}

.timing-type-group .el-radio {
    display: block;
    margin: 0;
    /* 移除默认的 margin */
}

.timing-type-group .el-radio__input {
    margin-right: 8px;
    /* 调整单选框与文本之间的间距 */
}

.timing-type-group .el-radio__label {
    line-height: 1;
    /* 确保文本的行高一致 */
}
</style>