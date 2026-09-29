def render(template, strict):
    return template if strict else template.strip()


async def _render_page(template):
    return render(template, False)


async def main(template):
    await _render_page(template)
    return render(template, True)
