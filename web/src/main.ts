import '@material/web/button/filled-tonal-button.js';
import '@material/web/button/outlined-button.js';
import '@material/web/button/text-button.js';
import '@material/web/checkbox/checkbox.js';
import '@material/web/chips/chip-set.js';
import '@material/web/chips/filter-chip.js';
import '@material/web/divider/divider.js';
import '@material/web/progress/circular-progress.js';
import '@material/web/textfield/outlined-text-field.js';

import { mount } from 'svelte';

import App from './App.svelte';
import './app.css';
import '$lib/workbench/workbench.css';

const app = mount(App, {
  target: document.getElementById('app')!
});

export default app;
