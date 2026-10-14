#!/usr/bin/env python3
"""Differential accuracy comparator: pdfrtl vs albdf (0.4.1) vs mupdf (mutool 1.27.0).

Compares extraction behavior, character counts, and reversal sanity across the 20 sample PDFs.
"""
import os, sys, json, pathlib, subprocess

ALBDF = "/home/amiralavi/playground/testing/bdf/oracles/bin/bin/albdf"
MUTOOL = "/usr/bin/mutool"
PDFRTL = "/home/amiralavi/playground/testing/bdf/target/debug/pdfrtl"
PDF_DIR = pathlib.Path("/Users/amiralavi/playground/testing/pdf_sample")

def is_reversed(text: str) -> bool:
    # Quick indicator check on common Persian/Hebrew reversed patterns
    # e.g. "رد" instead of "در", "هب" instead of "به", "تسا" instead of "است", "עדימ" instead of "מידע"
    hebrew_rev = "עדימ" in text or "תרבוח" in text or "ןוגראב" in text
    persian_rev = " هب " in text and not " به " in text
    return hebrew_rev or persian_rev

def run():
    pdfs = sorted(list(PDF_DIR.glob("*.pdf")))
    print(f"Comparing {len(pdfs)} PDFs across [pdfrtl, albdf 0.4.1, mutool 1.27.0]...\n")

    results = []
    for p in pdfs:
        name = p.name
        # 1. pdfrtl
        res_pdfrtl = subprocess.run([PDFRTL, "extract", "--json", str(p)], capture_output=True, text=True)
        pdfrtl_exit = res_pdfrtl.returncode
        try:
            pdfrtl_json = json.loads(res_pdfrtl.stdout)
            pdfrtl_chars = len(pdfrtl_json.get("data", {}).get("text", "") or "")
            pdfrtl_withheld = pdfrtl_json.get("data", {}).get("unordered_chars", 0)
            pdfrtl_sample = (pdfrtl_json.get("data", {}).get("text", "") or "").strip().splitlines()[:2]
        except:
            pdfrtl_chars = 0
            pdfrtl_withheld = 0
            pdfrtl_sample = []

        # 2. albdf (fetch-text page 1)
        res_al = subprocess.run([ALBDF, "fetch-text", "--page-first", "1", "--page-last", "1", str(p)], capture_output=True, text=True)
        albdf_p1_text = res_al.stdout.strip()
        albdf_chars = len(albdf_p1_text)
        albdf_sample = [l.strip() for l in albdf_p1_text.splitlines() if l.strip()][:2]
        albdf_rev = is_reversed(albdf_p1_text)

        # 3. mutool draw (page 1)
        res_mu = subprocess.run([MUTOOL, "draw", "-F", "txt", "-o", "-", str(p), "1"], capture_output=True, text=True)
        mutool_p1_text = res_mu.stdout.strip()
        mutool_chars = len(mutool_p1_text)
        mutool_sample = [l.strip() for l in mutool_p1_text.splitlines() if l.strip()][:2]
        mutool_rev = is_reversed(mutool_p1_text)

        results.append({
            "name": name,
            "pdfrtl_exit": pdfrtl_exit,
            "pdfrtl_chars": pdfrtl_chars,
            "pdfrtl_withheld": pdfrtl_withheld,
            "albdf_rev": albdf_rev,
            "mutool_rev": mutool_rev,
            "albdf_sample": " | ".join(albdf_sample)[:60],
            "mutool_sample": " | ".join(mutool_sample)[:60],
            "pdfrtl_sample": " | ".join(pdfrtl_sample)[:60],
        })

    # Print summary
    print(f"{'Filename':<35} | {'pdfrtl Exit':<11} | {'pdfrtl Chars':<12} | {'albdf p1 reversed?':<18} | {'mutool p1 reversed?'}")
    print("-" * 105)
    for r in results:
        al_r = "YES (REVERSED)" if r["albdf_rev"] else "No"
        mu_r = "YES (REVERSED)" if r["mutool_rev"] else "No"
        print(f"{r['name'][:35]:<35} | {r['pdfrtl_exit']:<11} | {r['pdfrtl_chars']:<12} | {al_r:<18} | {mu_r}")

    print("\nDetailed Qualitative Samples on Key Files:")
    for r in results:
        if r["name"] in ["חוברת.pdf", "قرارداد پشتیبانی و برون_سپاری خدمات فناوری اطلاعات (IT) اصلاح شده.pdf", "Children-rights-Persian.pdf"]:
            print(f"\n--- {r['name']} ---")
            print(f"  [pdfrtl] : {r['pdfrtl_sample']}")
            print(f"  [albdf]  : {r['albdf_sample']}")
            print(f"  [mutool] : {r['mutool_sample']}")

if __name__ == "__main__":
    run()
