<script lang="ts" setup>
import {
    Avatar,
    Button,
    DataTable,
    FloatLabel,
    InputNumber,
    Menubar,
    Panel,
    Select
} from "primevue";
import Chart from "primevue/chart";
import {onMounted, ref} from "vue";
import Column from 'primevue/column';
import {invoke} from '@tauri-apps/api/core';
import {UserInfo} from "./bo/user.ts";
import {LogLevel} from "./bo/common.ts";
import {LogInfoEvent} from "./bo/event.ts";
import {
    Configuration,
    ConnectionPoolConfiguration
} from "./bo/configuration.ts";

let agentServerPort = ref<number>(10080);
let workerThreadNumber = ref<number>(128);
let selectedMaxLogLevel = ref<LogLevel>(LogLevel.ERROR);
let connectionPoolCheckInterval = ref<number>(1000);
let connectionPoolFillInterval = ref<number>(1000);
let connectionPoolMaxPoolSize = ref<number>(100);
let selectedUser = ref<UserInfo>();
let userSelectOptions: UserInfo[] = [
    new UserInfo("User1", "127.0.0.1:10080", "user1"),
    new UserInfo("User2", "127.0.0.1:10081", "user2"),
    new UserInfo("User3", "127.0.0.1:10082", "user3")
];

async function startAgent() {
    let connectionPoolConfiguration = new ConnectionPoolConfiguration(
        connectionPoolCheckInterval.value,
        connectionPoolFillInterval.value,
        connectionPoolMaxPoolSize.value,
    );
    let configuration = new Configuration(
        agentServerPort.value,
        workerThreadNumber.value,
        selectedMaxLogLevel.value,
        connectionPoolConfiguration
    );
    await invoke("start_agent", {
        configuration
    });
    console.log(configuration)
}

function stopAgent() {
}

function importUsers() {
}

onMounted(() => {
    chartData.value = setChartData();
    chartOptions.value = setChartOptions();
    logEvents.value = [new LogInfoEvent(LogLevel.INFO, "log message 1"), new LogInfoEvent(LogLevel.INFO, "log message 2"), new LogInfoEvent(LogLevel.INFO, "log message 3")];
});
const chartData = ref();
const chartOptions = ref();
const logEvents = ref<LogInfoEvent[]>();
const setChartData = () => {
    const documentStyle = getComputedStyle(document.documentElement);
    return {
        labels: ['January', 'February', 'March', 'April', 'May', 'June', 'July'],
        datasets: [
            {
                label: 'Download',
                data: [65, 59, 80, 81, 56, 55, 40],
                fill: false,
                borderColor: documentStyle.getPropertyValue('--p-cyan-500'),
                tension: 0.4
            },
            {
                label: 'Upload',
                data: [28, 48, 40, 19, 86, 27, 90],
                fill: false,
                borderColor: documentStyle.getPropertyValue('--p-gray-500'),
                tension: 0.4
            }
        ]
    };
};
const setChartOptions = () => {
    const documentStyle = getComputedStyle(document.documentElement);
    const textColor = documentStyle.getPropertyValue('--p-text-color');
    const textColorSecondary = documentStyle.getPropertyValue('--p-text-muted-color');
    const surfaceBorder = documentStyle.getPropertyValue('--p-content-border-color');
    return {
        maintainAspectRatio: false,
        aspectRatio: 0.6,
        plugins: {
            legend: {
                labels: {
                    color: textColor
                }
            }
        },
        scales: {
            x: {
                ticks: {
                    color: textColorSecondary
                },
                grid: {
                    color: surfaceBorder
                }
            },
            y: {
                ticks: {
                    color: textColorSecondary
                },
                grid: {
                    color: surfaceBorder
                }
            }
        }
    };
}
</script>

<template>
    <Menubar>
        <template #start>
            <Avatar class="mr-3" icon="pi pi-user"
                    shape="circle" size="large"/>
            <h1 class="font-bold">Ppaass Agent</h1>
        </template>
        <template #end>
            <div v-if="selectedUser"
                 class="flex flex-col items-end">
                <div class="uppercase font-bold">Remote server</div>
                <div>{{ selectedUser.proxyAddress }}</div>
            </div>
        </template>

    </Menubar>
    <main
        class="p-3 min-w-120 grid grid-cols-1 gap-3">

        <div class="grid grid-cols-1 gap-3 md:grid-cols-2 lg:grid-cols-3">

            <Panel class="w-full" header="Select user">
                <FloatLabel variant="on">
                    <Select id="select_user" v-model="selectedUser"
                            :options="userSelectOptions"
                            class="w-full mt-3 mb-3"
                            option-label="nickname">
                        <template #dropdownicon>
                            <i class="pi pi-user"/>
                        </template>
                    </Select>
                    <label for="select_user">Select a user</label>
                </FloatLabel>
            </Panel>
            <Panel class="w-full"
                   header="Connection configuration">
                <FloatLabel variant="on">
                    <InputNumber id="agent_server_port"
                                 v-model="agentServerPort"
                                 :max="65535" :min="1"
                                 :useGrouping="false"
                                 class="w-full mt-3 mb-3"
                    ></InputNumber>
                    <label for="agent_server_port">Agent server port</label>
                </FloatLabel>
                <FloatLabel variant="on">
                    <InputNumber id="worker_thread_number"
                                 v-model="workerThreadNumber" :max="65535"
                                 :min="1"
                                 :useGrouping="false"
                                 class="w-full mt-3 mb-3"></InputNumber>
                    <label for="worker_thread_number">Worker thread
                        number</label>
                </FloatLabel>
                <FloatLabel variant="on">
                    <Select id="max_log_level"
                            v-model="selectedMaxLogLevel"
                            :options="[LogLevel.ERROR, LogLevel.INFO, LogLevel.WARNING, LogLevel.DEBUG]"
                            class="w-full mt-3 mb-3">
                        <template #dropdownicon>
                            <i class="pi pi-sliders-h"/>
                        </template>
                    </Select>
                    <label for="max_log_level">Max log level</label>
                </FloatLabel>

            </Panel>
            <Panel class="w-full md:col-span-2  lg:col-span-1"
                   header="Connection pool configuration">

                <FloatLabel variant="on">
                    <InputNumber id="max_pool_size"
                                 v-model="connectionPoolMaxPoolSize"
                                 :max="65535"
                                 :min="1"
                                 :useGrouping="false"
                                 class="w-full mt-3 mb-3"></InputNumber>
                    <label for="max_pool_size">Max pool size</label>
                </FloatLabel>
                <FloatLabel variant="on">
                    <InputNumber id="fill_interval"
                                 v-model="connectionPoolFillInterval"
                                 :max="60" :min="0" :useGrouping="false"
                                 class="w-full mt-3 mb-3"></InputNumber>
                    <label for="fill_interval">Fill interval</label>
                </FloatLabel>
                <FloatLabel variant="on">
                    <InputNumber id="check_interval"
                                 v-model="connectionPoolCheckInterval"
                                 :max="60" :min="0" :useGrouping="false"
                                 class="w-full mt-3 mb-3"></InputNumber>
                    <label for="check_interval">Check interval</label>
                </FloatLabel>
            </Panel>
        </div>

        <div class="grid grid-cols-1 gap-3">
            <Panel class="w-full"
                   header="Download/Upload speed" toggleable>
                <Chart :data="chartData" :options="chartOptions"
                       class="w-full h-[20rem]" type="line"/>
            </Panel>

            <Panel class="w-full" header="Log console" toggleable>
                <DataTable :value="logEvents" show-gridlines>
                    <Column class="w-1/12" field="level"
                            header="Level"></Column>
                    <Column field="message" header="Message"></Column>
                </DataTable>

            </Panel>
        </div>

        <div class="w-full flex flex-row justify-end gap-4 mt-5">
            <Button class="w-29 uppercase" label="Start"
                    @click="startAgent"></Button>
            <Button class="w-29 uppercase" label="Stop"
                    @click="stopAgent"></Button>
            <Button class="w-38 uppercase"
                    label="Import users" @click="importUsers"></Button>
        </div>


    </main>
</template>

<style>
:root {
    margin: 0;
    padding: 0;
    font-family: arial, helvetica, sans-serif, "Microsoft YaHei", '宋体';
}

</style>