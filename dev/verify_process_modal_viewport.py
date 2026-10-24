"""Ensure process editor modal remains above topbar and scrolls safely on narrow windows."""
import os
import unittest
from pathlib import Path
from playwright.sync_api import sync_playwright

BASE=os.getenv("PERCHFLOW_PREVIEW_URL","http://127.0.0.1:9060")
ROOT=Path(__file__).resolve().parents[1]

class ModalLayoutTests(unittest.TestCase):
    def test_modal_above_nav_and_scrollable(self):
        with sync_playwright() as p:
            browser=p.chromium.launch(channel="msedge",headless=True)
            try:
                for width,height in ((390,667),(506,768),(768,600),(1440,900)):
                    with self.subTest(width=width,height=height):
                        page=browser.new_page(viewport={"width":width,"height":height})
                        try:
                            page.goto(BASE+"/processes",wait_until="domcontentloaded")
                            page.locator('#proc-table-body tr').first.wait_for()
                            page.locator('tr[data-process-name="MailHub (DEMO)"] [data-action="edit"]').click()
                            page.locator('#add-modal').wait_for(state="visible")
                            layout=page.evaluate("""() => {
                              const modal=document.querySelector('#add-modal'), card=modal.firstElementChild;
                              const nav=document.querySelector('.pf-process-topnav');
                              const web=document.querySelector('#add-web-mode');
                              const rect=el=>{const r=el.getBoundingClientRect();return {top:r.top,bottom:r.bottom,left:r.left,right:r.right}};
                              return {modalZ:Number(getComputedStyle(modal).zIndex),
                                navZ:Number(getComputedStyle(nav).zIndex),
                                card:rect(card),cardHeight:card.clientHeight,scrollHeight:card.scrollHeight,
                                cardBackground:getComputedStyle(card).backgroundColor,
                                hoverMotion:getComputedStyle(card).transform,
                                webBackground:getComputedStyle(web).backgroundColor,
                                webAppearance:getComputedStyle(web).appearance,
                                webBackgroundImage:getComputedStyle(web).backgroundImage,
                                topHit:document.elementFromPoint(innerWidth/2,25)?.closest('#add-modal')?.id||'behind',
                                pageOverflow:document.documentElement.scrollWidth>window.innerWidth+1
                              };
                            }""")
                            assert layout['modalZ']>layout['navZ'],layout
                            assert layout['topHit']=='add-modal',layout
                            assert layout['card']['top']>=10,layout
                            assert layout['card']['bottom']<=height-10,layout
                            assert layout['cardBackground']!='rgba(255, 255, 255, 0.06)',layout
                            assert layout['webAppearance']=='none',layout
                            assert 'data:image/svg+xml' in layout['webBackgroundImage'],layout
                            assert not layout['pageOverflow'],layout
                            # Both selection modes and advanced panel must remain usable.
                            page.locator('#add-web-mode').select_option('manual')
                            assert not page.locator('#add-web-url').is_hidden()
                            page.locator('#add-web-mode').select_option('auto')
                            assert page.locator('#add-web-url').is_hidden()
                            page.locator('#add-advanced summary').click()
                            assert page.locator('#add-advanced').evaluate('(e)=>e.open')
                            page.locator('#add-restart-policy').select_option('never')
                            assert page.locator('#add-restart-policy').input_value()=='never'
                            page.locator('#add-modal button[onclick="submitAddModal()"]').scroll_into_view_if_needed()
                            assert page.locator('#add-modal button[onclick="submitAddModal()"]').is_visible()
                            expanded=page.evaluate("""() => {
                              let c=document.querySelector('.pf-process-edit-dialog'), b=document.querySelector('#add-modal button[onclick="submitAddModal()"]');
                              let r=b.getBoundingClientRect(),v=c.getBoundingClientRect();
                              return {scrollTop:c.scrollTop,scrollHeight:c.scrollHeight,clientHeight:c.clientHeight,buttonBottom:r.bottom,cardBottom:v.bottom};
                            }""")
                            assert expanded['buttonBottom']<=expanded['cardBottom']+1,expanded
                            print(f"PASS modal {width}x{height} nav={layout['navZ']} dialog={layout['modalZ']} card={round(layout['card']['top'])}-{round(layout['card']['bottom'])} scroll={expanded['scrollTop']}")
                        finally:page.close()
            finally:browser.close()

if __name__=="__main__":unittest.main()
