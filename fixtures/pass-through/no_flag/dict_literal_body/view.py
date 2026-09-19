def _card_view(card):
    return {
        "id": card.id,
        "front": card.front,
        "back": card.back,
    }


def render(cards):
    views = [_card_view(c) for c in cards]
    return views
