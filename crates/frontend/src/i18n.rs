leptos_i18n::declare_locales! {
    default: "en",
    locales: ["en", "ru"],

    en: {
        auth: {
            login: {
                title: "Login",
                username: "Username",
                code: "TOTP Code",
                button: "Login",
                switch_to_register: "Don't have an account? Register",
                error_empty: "Please enter username and code",
                error_invalid: "Invalid username or code",
            },
            register: {
                title: "Register",
                username: "Username",
                button: "Register",
                switch_to_login: "Already have an account? Login",
                success: "Registration successful! Scan the QR code with your authenticator app",
                qr_instruction: "Scan this QR code with Google Authenticator or similar app",
                back: "Continue to Login",
                error_empty: "Please enter username",
            },
            room: {
                title: "Select Room",
                room_id: "Room ID",
                button: "Connect",
                error_empty: "Please enter room ID",
            },
        },
        menu: {
            chat: "Chat",
            settings: "Settings",
            statistics: "Statistics",
        },
        settings: {
            title: "Settings",
            language: "Language",
            close: "Close",
        },
        statistics: {
            title: "Statistics",
            event_log: "Event Log",
            no_events: "No events yet",
            close: "Close",
        },
    },

    ru: {
        auth: {
            login: {
                title: "Вход",
                username: "Имя пользователя",
                code: "TOTP код",
                button: "Войти",
                switch_to_register: "Нет аккаунта? Зарегистрироваться",
                error_empty: "Пожалуйста, введите имя пользователя и код",
                error_invalid: "Неверное имя пользователя или код",
            },
            register: {
                title: "Регистрация",
                username: "Имя пользователя",
                button: "Зарегистрироваться",
                switch_to_login: "Уже есть аккаунт? Войти",
                success: "Регистрация успешна! Отсканируйте QR-код в приложении для аутентификации",
                qr_instruction: "Отсканируйте этот QR-код с помощью Google Authenticator или подобного приложения",
                back: "Назад к входу",
                error_empty: "Пожалуйста, введите имя пользователя",
            },
            room: {
                title: "Выбор комнаты",
                room_id: "ID комнаты",
                button: "Подключиться",
                error_empty: "Пожалуйста, введите ID комнаты",
            },
        },
        menu: {
            chat: "Чат",
            settings: "Настройки",
            statistics: "Статистика",
        },
        settings: {
            title: "Настройки",
            language: "Язык",
            close: "Закрыть",
        },
        statistics: {
            title: "Статистика",
            event_log: "Журнал событий",
            no_events: "Событий пока нет",
            close: "Закрыть",
        },
    },
}