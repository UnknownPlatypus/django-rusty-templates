from django import template

register = template.Library()


@register.filter
def square(value):
    return value * value
