from __future__ import annotations

from dataclasses import dataclass
from typing import Literal

from html_tstring import Fragment, RawHtml, render_html

type Theme = Literal["light", "dark"]


@dataclass(frozen=True)
class UserProfile:
    user_id: str
    display_name: str
    email: str
    is_pro: bool
    theme: Theme
    bio_html: str | None


def render_account_settings(profile: UserProfile) -> str:
    classes: list[object] = [
        "settings-card",
        f"theme-{profile.theme}",
        {"settings-card--pro": profile.is_pro},
    ]
    attrs: dict[str, object] = {
        "data-user-id": profile.user_id,
        "class": classes,
        "hidden": False,
    }
    bio_section: Fragment | str
    if profile.bio_html is None:
        bio_section = "No bio set."
    else:
        bio_section = Fragment(
            [
                RawHtml('<div class="settings-card__bio-label">Bio</div>'),
                RawHtml(profile.bio_html),
            ]
        )

    return render_html(t"""\
<section {attrs}>
  <header class="settings-card__header">
    <h1>{profile.display_name}</h1>
    <p>{profile.email}</p>
  </header>
  <div class="settings-card__body">
    {bio_section}
  </div>
</section>
""")


profile = UserProfile(
    user_id="user-42",
    display_name="Koudai Aono",
    email="koudai@example.com",
    is_pro=True,
    theme="dark",
    bio_html="<p>Maintainer of parser-first template tooling.</p>",
)

print(render_account_settings(profile))
# <section data-user-id="user-42" class="settings-card theme-dark settings-card--pro">
#   <header class="settings-card__header">
#     <h1>Koudai Aono</h1>
#     <p>koudai@example.com</p>
#   </header>
#   <div class="settings-card__body">
#     <div class="settings-card__bio-label">Bio</div><p>Maintainer of parser-first template tooling.</p>  # noqa: E501
#   </div>
# </section>
