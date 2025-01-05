from django import template

register = template.Library()


@register.filter
def cut(value, arg):
    """Removes all values of arg from the given string"""
    return value.replace(arg, "")


@register.filter
def double(value):
    return value * 2


@register.filter
def multiply(value, by=3):
    return value * by


@register.filter
def divide_by_zero(value, zero=0):
    return value / zero
