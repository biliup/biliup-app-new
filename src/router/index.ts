import { createRouter, createWebHistory } from 'vue-router'
import LoginView from '../components/LoginView.vue'
import MainView from '../views/MainView.vue'

const routes = [
    {
        path: '/',
        redirect: '/main'
    },
    {
        path: '/login',
        name: 'Login',
        component: LoginView
    },
    {
        path: '/main',
        name: 'Main',
        component: MainView
    }
]

const router = createRouter({
    history: createWebHistory(),
    routes
})

export default router
