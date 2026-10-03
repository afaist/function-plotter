# Function Plotter

Интерактивная программа для построения графиков математических функций.

![Linux](https://img.shields.io/badge/platform-linux-lightgrey)
![Windows](https://img.shields.io/badge/platform-windows-lightgrey)
![macOS](https://img.shields.io/badge/platform-macos-lightgrey)

## Возможности

- Построение графиков функций с адаптивной плотностью
- Поддержка производных: `deriv(sin(x))`
- Поддержка определённых интегралов: `integral(sin(x), 0, 3.14)`
- Масштабирование колесиком мыши и перемещение перетаскиванием
- Экспорт в CSV (один график или все) и PNG
- Сохранение и загрузка сессий в JSON
- Автоматическая загрузка последней сессии
- Поиск точек пересечения графиков

## Установка

### Сборка из исходного кода

```bash
git clone <repository-url>
cd function-plotter
cargo build --release
```

Готовый бинарный файл будет в `target/release/function-plotter`.

### Запуск

```bash
cargo run
```

## Использование

### Добавление графиков

1. Нажмите "+ Добавить график" в левой панели
2. Введите формулу в поле ввода (например: `sin(x)`, `x^2`)
3. Нажмите Enter или измените диапазон для пересчёта

### Горячие клавиши

| Клавиша | Действие |
|---------|----------|
| `Ctrl+N` | Новый график |
| `Ctrl+O` | Загрузить сессию |
| `Ctrl+S` | Сохранить сессию |
| `Delete` | Удалить выбранный график |
| `R` | Сбросить масштаб |
| `F5` / `Ctrl+Enter` | Пересчитать графики |

### Примеры функций

**Базовые:**
- `sin(x)`, `cos(x)`, `tan(x)`
- `x^2`, `x^3`, `sqrt(x)`
- `exp(x)`, `log(x)`, `abs(x)`
- `sin(x) / x`

**Производные:**
- `deriv(sin(x))` — производная sin(x)
- `deriv(x^3 + 2*x)` — производная полинома

**Интегралы:**
- `integral(sin(x), 0, 3.14)` — определённый интеграл
- `integral(x^2, 0, 1)` — площадь под кривой

### Управление видом

- **Масштабирование:** колесико мыши (масштаб к курсору)
- **Перемещение:** зажать левую кнопку мыши и тянуть
- **Авто-масштаб Y:** чекбокс "Авто-масштаб Y" в левой панели

### Экспорт

**CSV:**
- "Сохранить CSV (текущий график)" — экспорт одного графика в формате x,y
- "Сохранить CSV (все графики)" — экспорт всех графиков в формате x,y1,y2,...

**PNG:**
- "Сохранить PNG" — сохранение скриншота области графиков в формате PNG (область с осями и легендой)

### Сессии

**Сохранение:**
- "Сохранить сессию" — сохранение всех графиков и настроек в JSON
- `Ctrl+S` — быстрое сохранение в последний файл

**Загрузка:**
- "Загрузить сессию" — загрузка сессии из JSON файла
- `Ctrl+O` — быстрый выбор файла для загрузки

**Автозагрузка:**
- Чекбокс "Автозагрузка последней сессии" — автоматически загружает последнюю сессию при старте

## Формат сессии

Сессия сохраняется в JSON формате:

```json
{
  "version": 2,
  "graphs": [
    {
      "label": "f1",
      "formula": "sin(x)",
      "color": [100, 200, 255],
      "visible": true
    }
  ],
  "x_min": -10.0,
  "x_max": 10.0,
  "n_points": 500,
  "auto_y": true,
  "adaptive": false,
  "adaptive_tolerance": 0.01,
  "viewport_x_min": -10.0,
  "viewport_x_max": 10.0,
  "viewport_y_min": -5.0,
  "viewport_y_max": 5.0
}
```

## Настройки приложения

Конфигурация хранится в:
- **Linux:** `~/.local/share/function-plotter/config.json`
- **Windows:** `%APPDATA%\function-plotter\config.json`
- **macOS:** `~/Library/Application Support/function-plotter/config.json`

```json
{
  "version": 1,
  "window_width": 1000.0,
  "window_height": 700.0,
  "auto_load_last_session": false
}
```

## Сборка релизов

### Linux

```bash
cargo build --release --target x86_64-unknown-linux-gnu
```

### Windows

```bash
cargo build --release --target x86_64-pc-windows-msvc
```

### macOS

```bash
cargo build --release --target x86_64-apple-darwin
cargo build --release --target aarch64-apple-darwin
```

### GitHub Actions

Релизы автоматически собираются через GitHub Actions. Workflow настроен в `.github/workflows/release.yml`.

## Зависимости

- **eframe/egui** 0.36 — GUI фреймворк и скриншоты
- **serde/serde_json** — сериализация
- **rfd** — файловые диалоги
- **mathexpr** — математические выражения
- **image** — сохранение PNG
- **dirs** — определение пользовательских директорий

## Лицензия

MIT
