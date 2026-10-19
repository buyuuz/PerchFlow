"""Process Chinese alias regressions (no browser dependency)."""
from pathlib import Path
import unittest

ROOT=Path(__file__).resolve().parents[1]
FRONT=ROOT/"frontend"
BACK=ROOT/"src"
PAGE=(FRONT/"processes.html").read_text(encoding="utf-8")
JS=(FRONT/"assets/processes.js").read_text(encoding="utf-8")
CSS=(FRONT/"assets/process-table.css").read_text(encoding="utf-8")
DETAIL=(FRONT/"process-detail.html").read_text(encoding="utf-8")
DETAIL_JS=(FRONT/"assets/process-detail.js").read_text(encoding="utf-8")
DETAIL_CSS=(FRONT/"assets/process-detail.css").read_text(encoding="utf-8")
LANG=(FRONT/"assets/i18n.js").read_text(encoding="utf-8")
MODEL=(BACK/"config/model.rs").read_text(encoding="utf-8")
API=(BACK/"web/api_process.rs").read_text(encoding="utf-8")
MOCK=(ROOT/"dev/preview_server.py").read_text(encoding="utf-8")


class ProcessAliasTests(unittest.TestCase):
    def test_add_and_edit_alias_optional(self):
        self.assertIn('id="add-name"',PAGE)
        self.assertIn('id="add-alias-zh"',PAGE)
        self.assertIn('maxlength="80"',PAGE)
        self.assertIn("document.getElementById('add-alias-zh').value = ''",JS)
        self.assertIn("document.getElementById('add-alias-zh').value = p.alias_zh || ''",JS)
        self.assertIn("const alias_zh = document.getElementById('add-alias-zh').value.trim()",JS)
        self.assertIn("alias_zh: alias_zh || null",JS)

    def test_stacked_display_in_list_and_detail(self):
        self.assertIn('class="pf-process-name-stack"',JS)
        self.assertIn('class="pf-process-alias"',JS)
        self.assertIn('class="pf-process-name"',JS)
        self.assertIn(".pf-process-name-stack .pf-process-name",CSS)
        self.assertIn(".pf-process-alias",CSS)
        self.assertIn('id="proc-name"',DETAIL)
        self.assertIn('id="proc-alias-zh"',DETAIL)
        self.assertIn(".pf-detail-alias",DETAIL_CSS)
        self.assertIn("alias.hidden = !alias.textContent",DETAIL_JS)
        self.assertIn("String(p.alias_zh || '').toLocaleLowerCase()",JS)

    def test_api_optional_field_is_persisted_without_restarting(self):
        self.assertIn("pub alias_zh: Option<String>",MODEL)
        self.assertIn("alias_zh: Some(\"应用管理\".into())", (ROOT/"tests/config_test.rs").read_text(encoding="utf-8"))
        self.assertIn('skip_serializing_if = "Option::is_none"',MODEL)
        self.assertIn('alias_zh.as_deref(), Some("邮箱管理中心")',MODEL)
        self.assertIn("struct AliasUpdate",API)
        self.assertIn("/processes/:name/alias",API)
        self.assertIn("Some(p) => p.alias_zh = alias",API)
        self.assertIn("op_result(persist(&ctx).await)",API)
        self.assertIn("aliasChanged",JS)
        self.assertIn("base + '/alias'",JS)
        self.assertIn('pub alias_zh: Option<String>',API)
        self.assertIn('obj["alias_zh"] = alias',MOCK)

    def test_i18n_and_escaped_user_content(self):
        for key in ("label.alias_zh","label.name_en","placeholder.process_alias"):
            self.assertEqual(LANG.count("'"+key+"'"),2,key)
        self.assertIn("escapeHtml(p.alias_zh)",JS)
        self.assertIn("alias.textContent =",DETAIL_JS)


if __name__=="__main__":
    unittest.main()
