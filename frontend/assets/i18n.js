/**
 * Perch i18n — lightweight client-side internationalization.
 * Supports: zh, en, es, ja, pt
 */
(function () {
  const LANGS = ['zh', 'en', 'es', 'ja', 'pt'];
  const STORAGE_KEY = 'perch-lang';

  const translations = {
    // ---- Chinese (default) ----
    zh: {
      'nav.processes': '进程',
      'nav.tasks': '定时任务',
      'nav.processes_short': '进程',
      'nav.tasks_short': '任务',
      'button.add_process': '添加进程',
      'button.add_task': '添加任务',
      'button.enter': '进入',
      'button.save': '保存',
      'button.cancel': '取消',
      'button.stop': '停止',
      'button.restart': '重启',
      'button.start': '启动',
      'label.name': '名称',
      'label.command': '命令',
      'label.args': '参数 (空格分隔)',
      'label.autostart': '开机自启',
      'label.enabled': '启用',
      'label.cron': 'Cron 表达式',
      'label.cpu_usage': 'CPU 使用率',
      'label.memory': '内存占用',
      'label.uptime': '运行时间',
      'label.command_start': '启动命令',
      'label.schedule': '调度',
      'label.last_run': '上次运行',
      'label.pid': 'PID',
      'page.processes_title': '进程管理',
      'page.processes_subtitle': '监控和管理系统进程',
      'page.tasks_title': '定时任务',
      'page.tasks_subtitle': '管理 cron 调度的任务',
      'search.processes': '搜索进程...',
      'search.tasks': '搜索任务...',
      'login.token_label': '请输入 perch.toml 中配置的访问令牌',
      'log.output': '日志输出',
      'log.live': '实时',
      'log.lines_100': '100 行',
      'log.lines_500': '500 行',
      'log.lines_1000': '1000 行',
      'log.stream_ended': '— 日志流已结束 ({status}) —',
      'modal.add_process': '添加进程',
      'modal.add_task': '添加任务',
      'modal.edit_task': '编辑任务',
      'status.running': '运行中',
      'status.stopped': '已停止',
      'status.crashed': '已崩溃',
      'status.restarting': '重启中',
      'error.token_invalid': '令牌无效',
      'error.name_cmd_required': '名称和命令不能为空',
      'error.name_cmd_cron_required': '名称、命令和 cron 表达式不能为空',
      'error.save_failed': '保存失败',
      'lang.select': '语言'
    },

    // ---- English ----
    en: {
      'nav.processes': 'Processes',
      'nav.tasks': 'Tasks',
      'nav.processes_short': 'Processes',
      'nav.tasks_short': 'Tasks',
      'button.add_process': 'Add Process',
      'button.add_task': 'Add Task',
      'button.enter': 'Enter',
      'button.save': 'Save',
      'button.cancel': 'Cancel',
      'button.stop': 'Stop',
      'button.restart': 'Restart',
      'button.start': 'Start',
      'label.name': 'Name',
      'label.command': 'Command',
      'label.args': 'Arguments (space-separated)',
      'label.autostart': 'Auto-start',
      'label.enabled': 'Enabled',
      'label.cron': 'Cron Expression',
      'label.cpu_usage': 'CPU Usage',
      'label.memory': 'Memory',
      'label.uptime': 'Uptime',
      'label.command_start': 'Startup Command',
      'label.schedule': 'Schedule',
      'label.last_run': 'Last Run',
      'label.pid': 'PID',
      'page.processes_title': 'Process Management',
      'page.processes_subtitle': 'Monitor and manage system processes',
      'page.tasks_title': 'Scheduled Tasks',
      'page.tasks_subtitle': 'Manage cron-scheduled tasks',
      'search.processes': 'Search processes...',
      'search.tasks': 'Search tasks...',
      'login.token_label': 'Enter the access token configured in perch.toml',
      'log.output': 'Log Output',
      'log.live': 'Live',
      'log.lines_100': '100 lines',
      'log.lines_500': '500 lines',
      'log.lines_1000': '1000 lines',
      'log.stream_ended': '— Log stream ended ({status}) —',
      'modal.add_process': 'Add Process',
      'modal.add_task': 'Add Task',
      'modal.edit_task': 'Edit Task',
      'status.running': 'Running',
      'status.stopped': 'Stopped',
      'status.crashed': 'Crashed',
      'status.restarting': 'Restarting',
      'error.token_invalid': 'Invalid token',
      'error.name_cmd_required': 'Name and command are required',
      'error.name_cmd_cron_required': 'Name, command and cron expression are required',
      'error.save_failed': 'Save failed',
      'lang.select': 'Language'
    },

    // ---- Spanish ----
    es: {
      'nav.processes': 'Procesos',
      'nav.tasks': 'Tareas',
      'nav.processes_short': 'Procesos',
      'nav.tasks_short': 'Tareas',
      'button.add_process': 'Agregar Proceso',
      'button.add_task': 'Agregar Tarea',
      'button.enter': 'Entrar',
      'button.save': 'Guardar',
      'button.cancel': 'Cancelar',
      'button.stop': 'Detener',
      'button.restart': 'Reiniciar',
      'button.start': 'Iniciar',
      'label.name': 'Nombre',
      'label.command': 'Comando',
      'label.args': 'Argumentos (separados por espacios)',
      'label.autostart': 'Inicio automático',
      'label.enabled': 'Habilitado',
      'label.cron': 'Expresión Cron',
      'label.cpu_usage': 'Uso de CPU',
      'label.memory': 'Memoria',
      'label.uptime': 'Tiempo de actividad',
      'label.command_start': 'Comando de inicio',
      'label.schedule': 'Programación',
      'label.last_run': 'Última ejecución',
      'label.pid': 'PID',
      'page.processes_title': 'Gestión de Procesos',
      'page.processes_subtitle': 'Monitorear y gestionar procesos del sistema',
      'page.tasks_title': 'Tareas Programadas',
      'page.tasks_subtitle': 'Gestionar tareas programadas con cron',
      'search.processes': 'Buscar procesos...',
      'search.tasks': 'Buscar tareas...',
      'login.token_label': 'Ingrese el token de acceso configurado en perch.toml',
      'log.output': 'Salida de registro',
      'log.live': 'En vivo',
      'log.lines_100': '100 líneas',
      'log.lines_500': '500 líneas',
      'log.lines_1000': '1000 líneas',
      'log.stream_ended': '— Transmisión de registro finalizada ({status}) —',
      'modal.add_process': 'Agregar Proceso',
      'modal.add_task': 'Agregar Tarea',
      'modal.edit_task': 'Editar Tarea',
      'status.running': 'En ejecución',
      'status.stopped': 'Detenido',
      'status.crashed': 'Fallido',
      'status.restarting': 'Reiniciando',
      'error.token_invalid': 'Token inválido',
      'error.name_cmd_required': 'El nombre y el comando son obligatorios',
      'error.name_cmd_cron_required': 'Nombre, comando y expresión cron son obligatorios',
      'error.save_failed': 'Error al guardar',
      'lang.select': 'Idioma'
    },

    // ---- Japanese ----
    ja: {
      'nav.processes': 'プロセス',
      'nav.tasks': 'タスク',
      'nav.processes_short': 'プロセス',
      'nav.tasks_short': 'タスク',
      'button.add_process': 'プロセス追加',
      'button.add_task': 'タスク追加',
      'button.enter': 'ログイン',
      'button.save': '保存',
      'button.cancel': 'キャンセル',
      'button.stop': '停止',
      'button.restart': '再起動',
      'button.start': '開始',
      'label.name': '名前',
      'label.command': 'コマンド',
      'label.args': '引数 (スペース区切り)',
      'label.autostart': '自動開始',
      'label.enabled': '有効',
      'label.cron': 'Cron式',
      'label.cpu_usage': 'CPU使用率',
      'label.memory': 'メモリ',
      'label.uptime': '稼働時間',
      'label.command_start': '起動コマンド',
      'label.schedule': 'スケジュール',
      'label.last_run': '最終実行',
      'label.pid': 'PID',
      'page.processes_title': 'プロセス管理',
      'page.processes_subtitle': 'システムプロセスの監視と管理',
      'page.tasks_title': 'スケジュールタスク',
      'page.tasks_subtitle': 'cronスケジュールタスクを管理',
      'search.processes': 'プロセスを検索...',
      'search.tasks': 'タスクを検索...',
      'login.token_label': 'perch.tomlに設定されたアクセストークンを入力してください',
      'log.output': 'ログ出力',
      'log.live': 'ライブ',
      'log.lines_100': '100行',
      'log.lines_500': '500行',
      'log.lines_1000': '1000行',
      'log.stream_ended': '— ログストリームが終了しました ({status}) —',
      'modal.add_process': 'プロセス追加',
      'modal.add_task': 'タスク追加',
      'modal.edit_task': 'タスク編集',
      'status.running': '実行中',
      'status.stopped': '停止中',
      'status.crashed': 'クラッシュ',
      'status.restarting': '再起動中',
      'error.token_invalid': '無効なトークン',
      'error.name_cmd_required': '名前とコマンドは必須です',
      'error.name_cmd_cron_required': '名前、コマンド、Cron式は必須です',
      'error.save_failed': '保存に失敗しました',
      'lang.select': '言語'
    },

    // ---- Portuguese ----
    pt: {
      'nav.processes': 'Processos',
      'nav.tasks': 'Tarefas',
      'nav.processes_short': 'Processos',
      'nav.tasks_short': 'Tarefas',
      'button.add_process': 'Adicionar Processo',
      'button.add_task': 'Adicionar Tarefa',
      'button.enter': 'Entrar',
      'button.save': 'Salvar',
      'button.cancel': 'Cancelar',
      'button.stop': 'Parar',
      'button.restart': 'Reiniciar',
      'button.start': 'Iniciar',
      'label.name': 'Nome',
      'label.command': 'Comando',
      'label.args': 'Argumentos (separados por espaços)',
      'label.autostart': 'Início automático',
      'label.enabled': 'Habilitado',
      'label.cron': 'Expressão Cron',
      'label.cpu_usage': 'Uso de CPU',
      'label.memory': 'Memória',
      'label.uptime': 'Tempo de atividade',
      'label.command_start': 'Comando de inicialização',
      'label.schedule': 'Agendamento',
      'label.last_run': 'Última execução',
      'label.pid': 'PID',
      'page.processes_title': 'Gerenciamento de Processos',
      'page.processes_subtitle': 'Monitorar e gerenciar processos do sistema',
      'page.tasks_title': 'Tarefas Agendadas',
      'page.tasks_subtitle': 'Gerenciar tarefas agendadas com cron',
      'search.processes': 'Pesquisar processos...',
      'search.tasks': 'Pesquisar tarefas...',
      'login.token_label': 'Digite o token de acesso configurado em perch.toml',
      'log.output': 'Saída de log',
      'log.live': 'Ao vivo',
      'log.lines_100': '100 linhas',
      'log.lines_500': '500 linhas',
      'log.lines_1000': '1000 linhas',
      'log.stream_ended': '— Transmissão de log encerrada ({status}) —',
      'modal.add_process': 'Adicionar Processo',
      'modal.add_task': 'Adicionar Tarefa',
      'modal.edit_task': 'Editar Tarefa',
      'status.running': 'Em execução',
      'status.stopped': 'Parado',
      'status.crashed': 'Falhou',
      'status.restarting': 'Reiniciando',
      'error.token_invalid': 'Token inválido',
      'error.name_cmd_required': 'Nome e comando são obrigatórios',
      'error.name_cmd_cron_required': 'Nome, comando e expressão cron são obrigatórios',
      'error.save_failed': 'Falha ao salvar',
      'lang.select': 'Idioma'
    }
  };

  const langNames = { zh: '中文', en: 'English', es: 'Español', ja: '日本語', pt: 'Português' };
  const langFlags = { zh: '🇨🇳', en: '🇬🇧', es: '🇪🇸', ja: '🇯🇵', pt: '🇧🇷' };

  function detectLang() {
    const stored = localStorage.getItem(STORAGE_KEY);
    if (stored && translations[stored]) return stored;
    const browser = (navigator.language || 'en').toLowerCase().split('-')[0];
    return translations[browser] ? browser : 'en';
  }

  let currentLang = detectLang();

  function t(key, params) {
    let str = (translations[currentLang] && translations[currentLang][key])
      || (translations.en && translations.en[key])
      || key;
    if (params) {
      for (const [k, v] of Object.entries(params)) {
        str = str.replace(`{${k}}`, v);
      }
    }
    return str;
  }

  function getLang() { return currentLang; }

  function setLang(lang) {
    if (!translations[lang]) return;
    currentLang = lang;
    localStorage.setItem(STORAGE_KEY, lang);
    applyTranslations();
  }

  function applyTranslations() {
    document.documentElement.lang = currentLang;
    document.querySelectorAll('[data-i18n]').forEach(el => {
      const key = el.getAttribute('data-i18n');
      el.textContent = t(key);
    });
    document.querySelectorAll('[data-i18n-placeholder]').forEach(el => {
      const key = el.getAttribute('data-i18n-placeholder');
      el.setAttribute('placeholder', t(key));
    });
    document.querySelectorAll('[data-i18n-title]').forEach(el => {
      const key = el.getAttribute('data-i18n-title');
      el.setAttribute('title', t(key));
    });
    // Update language selector label
    document.querySelectorAll('[data-lang-current]').forEach(el => {
      el.textContent = langNames[currentLang];
    });
    // Update language selector value
    document.querySelectorAll('[data-lang-select]').forEach(el => {
      el.value = currentLang;
    });
  }

  // Expose to global scope
  window.t = t;
  window.getLang = getLang;
  window.setLang = setLang;
  window.applyTranslations = applyTranslations;
  window.I18N_LANGS = LANGS;
  window.I18N_langNames = langNames;
  window.I18N_langFlags = langFlags;

  // Auto-apply on DOM ready
  if (document.readyState === 'loading') {
    document.addEventListener('DOMContentLoaded', applyTranslations);
  } else {
    applyTranslations();
  }
})();
