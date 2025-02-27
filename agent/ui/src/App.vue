<script lang="ts" setup>
import {ref} from "vue";
import {invoke} from "@tauri-apps/api/core";
import InputText from 'primevue/inputtext';
import Button from "primevue/button";
import Panel from 'primevue/panel';
import Menu from "primevue/menu";
import {MenuItem} from "primevue/menuitem";

const greetMsg = ref("");
const name = ref("");
const menuItems: MenuItem[] = [
    {
        label: "Item1"
    },
    {
        label: "Item2"
    },
    {
        label: "Item3"
    }
]

async function greet() {
    // Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
    greetMsg.value = await invoke("greet", {name: name.value});
}


</script>

<template>
    <Menu :model="menuItems" class="h-full md:w-60"></Menu>
    <Panel class="container">
        <h1>Welcome to Tauri + Vue</h1>

        <Panel>Click on the Tauri, Vite, and Vue logos to learn more.</Panel>

        <form class="row">
            <InputText id="greet-input" v-model="name"
                       placeholder="Enter a name..."></InputText>
            <Button @click="greet">Greet</Button>
        </form>
        <Panel>
            {{ greetMsg }}
        </Panel>
    </Panel>
</template>
