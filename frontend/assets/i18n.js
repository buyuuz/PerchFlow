/**
 * Perch i18n — lightweight client-side internationalization.
 * User-selectable languages: zh, en; legacy translations retained for compatibility.
 */
(function () {
  const LANGS = ['zh', 'en'];
  const STORAGE_KEY = 'perch-lang';

  const translations = {
    // ---- Chinese (default) ----
    zh: {
      'nav.processes': '进程',
      'nav.tasks': '定时任务',
      'nav.projects': '项目',
      'nav.projects_short': '项目',
      'page.projects_title': '项目管理',
      'page.projects_subtitle': '关联现有任务、输入和结果文件；文件保持原位置',
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
      'label.name_en': '进程名称（英文）',
      'label.alias_zh': '中文别名（可选）',
      'label.alias_zh_hint': '仅用于显示，不影响进程的启动、停止和日志。',
      'placeholder.process_name': '例如：MailHub',
      'placeholder.process_alias': '例如：邮箱管理中心',
      'error.alias_too_long': '中文别名不能超过 80 个字符',
      'label.command': '命令',
      'label.args': '参数 (空格分隔)',
      'label.cwd': '工作目录（可选）',
      'label.cwd_hint': '相对路径以此目录为起点；留空时采用默认启动规则。更改运行中进程的目录会重启该进程。',
      'placeholder.cwd': '例如：D:\\Projects\\MyApp',
      'button.use_cwd_suggestion': '使用建议',
      'cwd.checking': '正在检查此目录是否存在…',
      'cwd.valid': '目录存在，可以使用',
      'error.cwd_not_absolute': '请输入绝对路径，例如 D:\\Projects\\MyApp',
      'error.cwd_not_found': '目录不存在，请检查路径或先创建文件夹',
      'error.cwd_not_directory': '填写的是文件，而不是文件夹',
      'error.cwd_inaccessible': '无法访问该目录，请检查路径与权限',
      'error.cwd_too_long': '工作目录不能超过 1024 个字符',
      'error.cwd_check_failed': '目录校验失败，未保存。请确认 PerchFlow 服务可访问',
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
      'log.clear': '清屏',
      'detail.back': '进程管理',
      'detail.label': '进程详情',
      'detail.copy_command': '复制启动命令',
      'detail.copied': '已复制',
      'detail.copy_failed': '复制失败',
      'detail.log_subtitle': '查看进程输出与运行记录',
      'detail.log_footer': '标准输出 · 仅当前进程',
      'detail.log_count': '{count} 行',
      'detail.no_logs': '等待日志输出…',
      'detail.live_status': '实时模式',
      'detail.paused_status': '已暂停',
      'detail.switch_live': '切换实时日志',
      'modal.add_process': '添加进程',
      'modal.edit_process': '编辑进程',
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
      'confirm.delete_process': '确定要删除此进程吗？',
      'confirm.delete_task': '确定要删除此任务吗？',
      'lang.select': '语言'
    },

    // ---- English ----
    en: {
      'nav.processes': 'Processes',
      'nav.tasks': 'Tasks',
      'nav.projects': 'Projects',
      'nav.projects_short': 'Projects',
      'page.projects_title': 'Projects',
      'page.projects_subtitle': 'Link tasks and input/output files without moving them',
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
      'label.name_en': 'Process name (English)',
      'label.alias_zh': 'Chinese alias (optional)',
      'label.alias_zh_hint': 'Display only. Process commands and logs still use the English name.',
      'placeholder.process_name': 'e.g. MailHub',
      'placeholder.process_alias': 'e.g. 邮箱管理中心',
      'error.alias_too_long': 'Chinese alias must be 80 characters or fewer',
      'label.command': 'Command',
      'label.args': 'Arguments (space-separated)',
      'label.cwd': 'Working directory (optional)',
      'label.cwd_hint': 'Relative paths start here. Leave empty for default behavior. Changing the directory of a running process restarts it.',
      'placeholder.cwd': 'e.g. D:\\Projects\\MyApp',
      'button.use_cwd_suggestion': 'Use suggestion',
      'cwd.checking': 'Checking the directory on the server…',
      'cwd.valid': 'Directory exists and is accessible',
      'error.cwd_not_absolute': 'Enter an absolute path such as D:\\Projects\\MyApp',
      'error.cwd_not_found': 'Directory does not exist. Check the path or create the folder.',
      'error.cwd_not_directory': 'The path points to a file, not a folder.',
      'error.cwd_inaccessible': 'Cannot access this directory. Check its path and permissions.',
      'error.cwd_too_long': 'Working directory must be 1024 characters or fewer.',
      'error.cwd_check_failed': 'Directory check failed. Nothing was saved; check the PerchFlow service.',
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
      'log.clear': 'Clear',
      'detail.back': 'Processes',
      'detail.label': 'Process details',
      'detail.copy_command': 'Copy command',
      'detail.copied': 'Copied',
      'detail.copy_failed': 'Copy failed',
      'detail.log_subtitle': 'Process output and execution activity',
      'detail.log_footer': 'Standard output · this process only',
      'detail.log_count': '{count} lines',
      'detail.no_logs': 'Waiting for log output…',
      'detail.live_status': 'Live mode',
      'detail.paused_status': 'Paused',
      'detail.switch_live': 'Toggle live logs',
      'modal.add_process': 'Add Process',
      'modal.edit_process': 'Edit Process',
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
      'confirm.delete_process': 'Are you sure you want to delete this process?',
      'confirm.delete_task': 'Are you sure you want to delete this task?',
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
      'log.clear': 'Limpiar',
      'modal.add_process': 'Agregar Proceso',
      'modal.edit_process': 'Editar Proceso',
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
      'confirm.delete_process': '¿Está seguro de que desea eliminar este proceso?',
      'confirm.delete_task': '¿Está seguro de que desea eliminar esta tarea?',
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
      'log.clear': 'クリア',
      'modal.add_process': 'プロセス追加',
      'modal.edit_process': 'プロセス編集',
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
      'confirm.delete_process': 'このプロセスを削除してもよろしいですか？',
      'confirm.delete_task': 'このタスクを削除してもよろしいですか？',
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
      'log.clear': 'Limpar',
      'modal.add_process': 'Adicionar Processo',
      'modal.edit_process': 'Editar Processo',
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
      'confirm.delete_process': 'Tem certeza que deseja excluir este processo?',
      'confirm.delete_task': 'Tem certeza que deseja excluir esta tarefa?',
      'lang.select': 'Idioma'
    }
  };

  const langNames = { zh: '中文', en: 'English', es: 'Español', ja: '日本語', pt: 'Português' };
  const langFlags = { zh: '🇨🇳', en: '🇬🇧', es: '🇪🇸', ja: '🇯🇵', pt: '🇧🇷' };

  function detectLang() {
    const stored = localStorage.getItem(STORAGE_KEY);
    if (stored && LANGS.includes(stored)) return stored;
    return 'en';
  }

  // Track whether this browser had an explicit choice at startup, so the
  // async server fetch below only overrides the default when the user has
  // never picked a language on this browser.
  const hadStoredLang = !!localStorage.getItem(STORAGE_KEY);

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
    if (!LANGS.includes(lang)) return;
    currentLang = lang;
    localStorage.setItem(STORAGE_KEY, lang);
    applyTranslations();
    document.dispatchEvent(new CustomEvent('perchflow:language-changed', { detail: {lang: currentLang} }));
    const token = localStorage.getItem('perch_token');
    if (token) {
      fetch('/api/lang', {
        method: 'PUT',
        headers: { 'Authorization': 'Bearer ' + token, 'Content-Type': 'application/json' },
        body: JSON.stringify({ lang })
      }).catch(() => {});
    }
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

  // Reconcile with the server-persisted language (GET /api/lang is
  // unauthenticated so this works pre-login too). Only adopt it when this
  // browser has never made an explicit choice, so a local override always wins.
  if (!hadStoredLang) {
    fetch('/api/lang').then(r => r.json()).then(data => {
      if (data && data.lang && LANGS.includes(data.lang) && data.lang !== currentLang) {
        setLang(data.lang);
      }
    }).catch(() => {});
  }
})();
