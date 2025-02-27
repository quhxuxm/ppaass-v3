import {createApp} from "vue";
import App from "./App.vue";
import PrimeVue from 'primevue/config';
import Aura from '@primeuix/themes/aura';
import Button from "primevue/button";

let app = createApp(App);
app.use(PrimeVue, {
    theme: {
        preset: Aura
    }
});
app.mount("#app");
app.component('Button', Button);

