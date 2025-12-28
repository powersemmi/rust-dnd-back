#!/bin/sh
# Скрипт для загрузки переменных из .env файла в Docker
set -e

ENV_FILE="${1:-.env}"

if [ ! -f "$ENV_FILE" ]; then
    echo "Error: $ENV_FILE not found"
    exit 1
fi

# Читаем файл построчно и экспортируем переменные
while IFS= read -r line || [ -n "$line" ]; do
    # Пропускаем пустые строки и комментарии
    case "$line" in
        ''|\#*) continue ;;
    esac

    # Экспортируем переменную
    export "$line"
done < "$ENV_FILE"

# Сдвигаем аргументы, чтобы убрать путь к .env файлу
shift

# Выполняем команду переданную как оставшиеся аргументы
exec "$@"
